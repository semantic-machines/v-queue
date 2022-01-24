use crate::queue::*;
use crate::record::*;
use crc32fast::Hasher;
use fs2::FileExt;
use std::cmp::Ordering;
use std::fs::*;
use std::io::prelude::*;
use std::io::SeekFrom;
use std::io::{BufRead, BufReader};

pub struct Consumer {
    mode: Mode,
    pub name: String,
    pub queue: Queue,
    pub count_popped: u32,
    pub id: u32,

    is_ready: bool,
    pos_record: u64,
    ff_info_pop: File,
    base_path: String,

    // tmp
    pub header: Header,
    hash: Hasher,
}

impl Drop for Consumer {
    fn drop(&mut self) {
        if self.mode != Mode::Read {
            let info_name_lock = self.base_path.to_owned() + "/" + &self.queue.name + "_info_pop_" + &self.name + ".lock";
            if let Err(e) = remove_file(&info_name_lock) {
                error!("[queue:consumer] drop: queue:{}:{}:{}, fail remove lock file {}, err={:?}", self.queue.name, self.queue.id, self.name, &info_name_lock, e);
            }
        }
    }
}

impl Consumer {
    pub fn new(base_path: &str, consumer_name: &str, queue_name: &str) -> Result<Consumer, ErrorQueue> {
        Consumer::new_with_mode(base_path, consumer_name, queue_name, Mode::ReadWrite)
    }

    pub fn new_with_mode(base_path: &str, consumer_name: &str, queue_name: &str, mode: Mode) -> Result<Consumer, ErrorQueue> {
        let info_name = base_path.to_owned() + "/" + queue_name + "_info_pop_" + consumer_name;

        match Queue::new(base_path, queue_name, Mode::Read) {
            Ok(q) => {
                if mode == Mode::ReadWrite {
                    let info_name_lock = base_path.to_owned() + "/" + queue_name + "_info_pop_" + consumer_name + ".lock";

                    match OpenOptions::new().read(true).write(true).create(true).open(info_name_lock) {
                        Ok(file) => {
                            if let Err(e) = file.lock_exclusive() {
                                error!("consumer:{} attempt lock, err={}", consumer_name, e);
                                return Err(ErrorQueue::AlreadyOpen);
                            }
                        },
                        Err(e) => {
                            error!("consumer:{} prepare lock, err={}", consumer_name, e);
                            return Err(ErrorQueue::FailOpen);
                        },
                    }
                }

                let open_with_option = if mode == Mode::ReadWrite {
                    OpenOptions::new().read(true).write(true).create(true).open(info_name)
                } else {
                    OpenOptions::new().read(true).open(info_name)
                };

                match open_with_option {
                    Ok(ff) => Ok({
                        let mut consumer = Consumer {
                            mode,
                            is_ready: true,
                            name: consumer_name.to_owned(),
                            ff_info_pop: ff,
                            queue: q,
                            count_popped: 0,
                            pos_record: 0,
                            hash: Hasher::new(),
                            header: Header {
                                start_pos: 0,
                                msg_length: 0,
                                magic_marker: 0,
                                count_pushed: 0,
                                crc: 0,
                                msg_type: MsgType::String,
                            },
                            base_path: base_path.to_string(),
                            id: 0,
                        };

                        if consumer.get_info() {
                            if consumer.queue.open_part(consumer.id).is_ok() {
                                if consumer.queue.ff_queue.seek(SeekFrom::Start(consumer.pos_record)).is_err() {
                                    return Err(ErrorQueue::NotReady);
                                }
                            } else {
                                consumer.queue.is_ready = true;
                                consumer.id = consumer.queue.id;
                                if consumer.queue.open_part(consumer.id).is_ok() {
                                    if consumer.queue.ff_queue.seek(SeekFrom::Start(consumer.pos_record)).is_err() {
                                        return Err(ErrorQueue::NotReady);
                                    }
                                } else {
                                    return Err(ErrorQueue::NotReady);
                                }
                            }
                        } else {
                            return Err(ErrorQueue::NotReady);
                        }

                        consumer
                    }),
                    Err(_e) => Err(ErrorQueue::NotReady),
                }
            },
            Err(_e) => Err(ErrorQueue::NotReady),
        }
    }

    pub fn get_batch_size(&mut self) -> u32 {
        self.get_batch_size_l(0)
    }

    fn get_batch_size_l(&mut self, level: u16) -> u32 {
        if self.queue.count_pushed < self.count_popped {
            return 0;
        }

        let delta = self.queue.count_pushed - self.count_popped;
        //eprintln!("get_batch_size queue.count_pushed={}, count_popped={}", self.queue.count_pushed, self.count_popped);
        //eprintln!("delta={}", delta);
        match delta.cmp(&0) {
            Ordering::Equal => {
                // if not new messages, read queue info
                self.queue.get_info_queue();

                if self.queue.id > self.id {
                    if level == 0 && self.go_to_next_part() {
                        return self.get_batch_size_l(level + 1);
                    }

                    return 0;
                }
            },
            Ordering::Greater => {
                if self.queue.id != self.id {
                    if level == 0 && self.go_to_next_part() {
                        return self.get_batch_size_l(level + 1);
                    }

                    return 0;
                } else {
                    return delta;
                }
            },
            _ => {},
        }
        0
    }

    pub fn open(&mut self, is_new: bool) -> bool {
        if !self.queue.is_ready {
            error!("[queue:consumer] open: queue not ready, set consumer.ready = false");
            self.is_ready = false;
            return false;
        }

        let info_pop_file_name = self.queue.base_path.to_owned() + "/" + &self.queue.name + "_info_pop_" + &self.name;

        let open_with_option = if self.mode == Mode::ReadWrite {
            OpenOptions::new().read(true).write(true).truncate(true).create(is_new).open(&info_pop_file_name)
        } else {
            OpenOptions::new().read(true).open(&info_pop_file_name)
        };

        if let Ok(ff) = open_with_option {
            self.ff_info_pop = ff;
        } else {
            error!("[queue:consumer] open: fail open file [{}], set consumer.ready = false", info_pop_file_name);
            self.is_ready = false;
            return false;
        }
        true
    }

    pub fn get_info(&mut self) -> bool {
        let mut res = true;

        if self.ff_info_pop.seek(SeekFrom::Start(0)).is_err() {
            return false;
        }

        if let Some(line) = BufReader::new(&self.ff_info_pop).lines().next() {
            if let Ok(ll) = line {
                if let Ok((queue_name, consumer_name, position, count_popped, id)) = scan_fmt!(&ll, "{};{};{};{};{}", String, String, u64, u32, u32) {
                    if queue_name != self.queue.name {
                        res = false;
                    }

                    if consumer_name != self.name {
                        res = false;
                    }

                    self.pos_record = position;
                    self.count_popped = count_popped;
                    self.id = id;
                } else {
                    res = false;
                }
            } else {
                return false;
            }
        }

        debug!("[queue:consumer] ({}): count_pushed:{}, position:{}, id:{}, success:{}", self.name, self.count_popped, self.pos_record, self.id, res);
        res
    }

    pub fn pop_header(&mut self) -> bool {
        let res = self.read_header();

        if !res {
            self.sync_and_set_cur_pos();
        }

        res
    }

    pub fn go_to_next_part(&mut self) -> bool {
        if self.count_popped >= self.queue.count_pushed {
            if let Err(e) = self.queue.get_info_of_part(self.id, false) {
                error!("{}, queue:consumer({}):pop, queue {}{} not ready", e.as_str(), self.name, self.queue.name, self.id);
                return false;
            }
        }

        if self.count_popped >= self.queue.count_pushed {
            //info!("@end of part {}, queue.id={}", self.id, self.queue.id);

            if self.queue.id == self.id {
                self.queue.get_info_queue();
            }

            if self.queue.id > self.id {
                while self.id < self.queue.id {
                    self.id += 1;

                    debug!("prepare next part {}", self.id);

                    if let Err(e) = self.queue.get_info_of_part(self.id, false) {
                        if e == ErrorQueue::NotFound {
                            warn!("queue:consumer({}):pop, queue {}:{} {}", self.name, self.queue.name, self.id, e.as_str());
                        } else {
                            error!("queue:consumer({}):pop, queue {}:{} {}", self.name, self.queue.name, self.id, e.as_str());
                            return false;
                        }
                    } else {
                        warn!("use next part {}", self.id);
                        break;
                    }
                }

                self.count_popped = 0;
                self.pos_record = 0;

                self.open(true);
                self.commit();

                if let Err(e) = self.queue.open_part(self.id) {
                    error!("queue:consumer({}):pop, queue {}:{}, open part: {}", self.name, self.queue.name, self.id, e.as_str());
                }
                return true;
            }
        }
        false
    }

    fn read_header(&mut self) -> bool {
        self.go_to_next_part();
        let mut buf = vec![0; HEADER_SIZE];
        match self.queue.ff_queue.read(&mut buf[..]) {
            Ok(len) => {
                //println!("@len={}, id={}", len, self.id);
                if len < HEADER_SIZE {
                    //self.is_ready = false;
                    //error!("fail read message header: len={}", len);
                    return false;
                }
            },
            Err(_) => {
                error!("[queue:consumer] fail read message header");
                //self.is_ready = false;
                return false;
            },
        }

        let header = Header::create_from_buf(&buf);

        if header.count_pushed > self.queue.count_pushed {
            if header.magic_marker != MAGIC_MARKER {
                error!("[queue:consumer] header is invalid: not found magic marker");
                self.seek_next_pos();
            } else {
                error!("[queue:consumer] header is invalid: record header count_pushed {} > queue count pushed {}", header.count_pushed, self.queue.count_pushed);
            }
            return false;
        }

        if header.start_pos >= self.queue.right_edge {
            error!("[queue:consumer] header is invalid");
            return false;
        }

        buf[21] = 0;
        buf[22] = 0;
        buf[23] = 0;
        buf[24] = 0;

        self.hash = Hasher::new();
        self.hash.update(&buf[..]);

        self.header = header;
        true
    }

    pub fn seek_next_pos(&mut self) -> bool {
        let from = self.header.start_pos + HEADER_SIZE as u64;
        warn!("[queue:consumer] abnormal situation: seek next record, from pos={}", from);
        if let Err(e) = self.queue.ff_queue.seek(SeekFrom::Start(from)) {
            error!("[queue:consumer] fail seek in queue, err={:?}", e);
            return false;
        }

        let mut buf = vec![0; 65536];
        let mut sbi = 0;
        match self.queue.ff_queue.read(&mut buf[..]) {
            Ok(_len) => {
                for (idx, b) in buf.iter().enumerate() {
                    if *b == MAGIC_MARKER_BYTES[sbi] {
                        sbi += 1;
                        if sbi >= 3 {
                            let new_pos = from + idx as u64 - 14;
                            if let Err(e) = self.queue.ff_queue.seek(SeekFrom::Start(new_pos)) {
                                error!("[queue:consumer] fail seek in queue, err={:?}", e);
                                return false;
                            }
                            warn!("[queue:consumer] next record pos={}, delta={}", new_pos, new_pos - self.header.start_pos);
                            self.pos_record = new_pos;
                            self.is_ready = true;
                            self.count_popped += 1;
                            return true;
                        }
                    } else {
                        sbi = 0;
                    }
                }
                return false;
            },
            Err(_) => {
                error!("[queue:consumer] seek_next_pos: fail to read queue");
                //self.is_ready = false;
                return false;
            },
        }
    }

    fn sync_and_set_cur_pos(&mut self) {
        if let Err(e) = self.queue.ff_queue.sync_data() {
            error!("[queue:consumer] fail sync data, err={:?}", e);
        }
        if let Err(e) = self.queue.ff_queue.seek(SeekFrom::Start(self.pos_record)) {
            error!("[queue:consumer] fail seek in queue, err={:?}", e);
        }
    }

    pub fn pop_body(&mut self, msg: &mut [u8]) -> Result<usize, ErrorQueue> {
        if !self.is_ready {
            return Err(ErrorQueue::NotReady);
        }

        if let Ok(readed_size) = self.queue.ff_queue.read(msg) {
            if readed_size != msg.len() {
                if self.count_popped == self.queue.count_pushed {
                    warn!("[queue:consumer] detected problem with 'Read Tail Message': size fail");

                    if self.queue.ff_queue.seek(SeekFrom::Start(self.pos_record)).is_ok() {
                        return Err(ErrorQueue::FailReadTailMessage);
                    }
                }
                return Err(ErrorQueue::FailRead);
            }

            //debug!("msg={:?}", msg);

            self.pos_record = self.pos_record + HEADER_SIZE as u64 + readed_size as u64;
            self.hash.update(msg);

            let crc32: u32 = self.hash.clone().finalize();

            if crc32 != self.header.crc {
                if self.count_popped == self.queue.count_pushed {
                    warn!("[queue:consumer] detected problem with 'Read Tail Message': CRC fail");

                    if self.queue.ff_queue.seek(SeekFrom::Start(self.pos_record)).is_ok() {
                        return Err(ErrorQueue::FailReadTailMessage);
                    }
                }

                error!("[queue:consumer] CRC fail, pos={}, record size={}", self.header.start_pos, self.header.msg_length + HEADER_SIZE as u32);
                self.is_ready = false;
                return Err(ErrorQueue::InvalidChecksum);
            }
            Ok(readed_size)
        } else {
            Err(ErrorQueue::FailRead)
        }
    }

    pub fn commit(&mut self) {
        if self.ff_info_pop.seek(SeekFrom::Start(0)).is_err() {
            error!("[queue:consumer] fail put info, set consumer.ready = false");
            self.is_ready = false;
        }
        if self.ff_info_pop.write(format!("{};{};{};{};{}\n", self.queue.name, self.name, self.pos_record, self.count_popped, self.id).as_bytes()).is_err() {
            error!("[queue:consumer] fail put info, set consumer.ready = false");
            self.is_ready = false;
        }
    }

    pub fn next(&mut self, commit: bool) -> bool {
        if !self.is_ready {
            error!("[queue:consumer] commit");
            return false;
        }

        self.count_popped += 1;

        if commit {
            if self.ff_info_pop.seek(SeekFrom::Start(0)).is_err() {
                return false;
            }

            if self.ff_info_pop.write(format!("{};{};{};{};{}\n", self.queue.name, self.name, self.pos_record, self.count_popped, self.id).as_bytes()).is_ok() {
                return true;
            };

            return false;
        }

        true
    }

    pub fn commit_and_next(&mut self) -> bool {
        self.next(true)
    }
}
