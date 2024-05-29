use crate::consumer::Consumer;
use crate::queue::Queue;
use crate::record::{ErrorQueue, Mode, MsgType};
use std::time::Duration;
use std::{fs, thread};

fn check_message_integrity(received_numbers: &[i32]) {
    println!("{:?}", received_numbers);
    for i in 1..received_numbers.len() {
        assert_eq!(received_numbers[i], received_numbers[i - 1] + 1);
    }
}

#[test]
fn test_read_only_mode() {
    let base_path = create_unique_queue_path("./test-tmp", "queue");
    let queue_name = "test_queue";

    // Создаем очередь в режиме ReadWrite и записываем сообщения
    let mut queue = Queue::new(&base_path, queue_name, Mode::ReadWrite).unwrap();
    let num_messages = 5;
    for i in 0..num_messages {
        let msg = format!("Message {}", i);
        queue.push(msg.as_bytes(), MsgType::String).unwrap();
    }

    // Закрываем очередь и открываем ее в режиме ReadOnly
    drop(queue);
    let mut queue = Queue::new(&base_path, queue_name, Mode::Read).unwrap();

    // Пытаемся записать сообщение в очередь
    let message = "Hello, world!".as_bytes();
    let msg_type = MsgType::String;
    let result = queue.push(message, msg_type);

    // Проверяем, что запись не производится и возвращается ошибка Other
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), ErrorQueue::NotReady);

    // Создаем потребителя и проверяем, что сообщения доступны для чтения
    let consumer_name = "consumer";
    let mut consumer = Consumer::new(&base_path, consumer_name, queue_name).unwrap();
    let mut received_messages = Vec::new();
    while consumer.pop_header() {
        let msg_size = consumer.header.msg_length as usize;
        let mut msg = vec![0; msg_size];
        if consumer.pop_body(&mut msg).is_ok() {
            let message = String::from_utf8(msg).unwrap();
            received_messages.push(message);
            consumer.commit();
        } else {
            break;
        }
    }

    // Проверяем, что все сообщения были прочитаны
    assert_eq!(received_messages.len(), num_messages as usize);
    for (i, msg) in received_messages.iter().enumerate() {
        assert_eq!(msg, &format!("Message {}", i));
    }
}

#[test]
fn test_queue_consumer_interaction() {
    let base_path = create_unique_queue_path("./test-tmp", "queue");
    let queue_name = "test_queue";

    // Создаем очередь и записываем сообщения с номерами
    let mut queue = Queue::new(&base_path, queue_name, Mode::ReadWrite).unwrap();
    let num_messages = 10;
    for i in 0..num_messages {
        let msg = format!("{}", i);
        queue.push(msg.as_bytes(), MsgType::String).unwrap();
    }

    // Создаем несколько потребителей
    let num_consumers = 3;
    let mut consumers = Vec::new();
    for i in 0..num_consumers {
        let consumer_name = format!("consumer_{}", i);
        let consumer = Consumer::new(&base_path, &consumer_name, queue_name).unwrap();
        consumers.push(consumer);
    }

    // Каждый потребитель читает свою часть сообщений
    let mut received_numbers = vec![Vec::new(); num_consumers];
    for (i, consumer) in consumers.iter_mut().enumerate() {
        while consumer.pop_header() {
            let msg_size = consumer.header.msg_length as usize;
            let mut msg = vec![0; msg_size];
            if consumer.pop_body(&mut msg).is_ok() {
                let number = String::from_utf8(msg).unwrap().parse::<i32>().unwrap();
                received_numbers[i].push(number);
                consumer.commit();
            } else {
                break;
            }
        }
    }

    // Проверяем, что каждый потребитель получил свою часть сообщений без пропусков
    for numbers in received_numbers.iter() {
        check_message_integrity(numbers);
    }

    // Закрываем очередь и открываем ее снова для создания нового сегмента
    drop(queue);
    let mut queue = Queue::new(&base_path, queue_name, Mode::ReadWrite).unwrap();

    // Записываем новые сообщения с номерами в очередь
    let num_new_messages = 5;
    for i in num_messages..num_messages + num_new_messages {
        let msg = format!("{}", i);
        queue.push(msg.as_bytes(), MsgType::String).unwrap();
    }

    // Ждем, пока потребители прочитают новые сообщения
    thread::sleep(Duration::from_millis(100));

    // Каждый потребитель читает свою часть новых сообщений
    let mut new_received_numbers = vec![Vec::new(); num_consumers];
    for (i, consumer) in consumers.iter_mut().enumerate() {
        while consumer.pop_header() {
            let msg_size = consumer.header.msg_length as usize;
            let mut msg = vec![0; msg_size];
            if consumer.pop_body(&mut msg).is_ok() {
                let number = String::from_utf8(msg).unwrap().parse::<i32>().unwrap();
                new_received_numbers[i].push(number);
                consumer.commit();
            } else {
                break;
            }
        }
    }

    // Проверяем, что каждый потребитель получил свою часть новых сообщений без пропусков
    for numbers in new_received_numbers.iter() {
        check_message_integrity(numbers);
    }
}

#[test]
fn test_consumer_reconnect() {
    println!("test_consumer_reconnect");
    let base_path = create_unique_queue_path("./test-tmp", "queue");
    let queue_name = "test_queue";

    // Создаем очередь и записываем сообщения
    let mut queue = Queue::new(&base_path, queue_name, Mode::ReadWrite).unwrap();
    let num_messages = 10;
    for i in 0..num_messages {
        let msg = format!("{}", i);
        println!("push {}", msg);
        queue.push(msg.as_bytes(), MsgType::String).unwrap();
    }

    // Создаем потребителя и читаем часть сообщений
    let consumer_name = "consumer";
    let mut consumer = Consumer::new(&base_path, consumer_name, queue_name).unwrap();
    let num_read_messages = 5;
    for _ in 0..num_read_messages {
        if consumer.pop_header() {
            let msg_size = consumer.header.msg_length as usize;
            let mut msg = vec![0; msg_size];
            consumer.pop_body(&mut msg).unwrap();
            consumer.commit();
        }
    }

    // Закрываем потребителя и создаем нового с тем же именем
    drop(consumer);
    let mut consumer = Consumer::new(&base_path, consumer_name, queue_name).unwrap();

    // Читаем оставшиеся сообщения
    let mut received_numbers = Vec::new();
    while consumer.pop_header() {
        let msg_size = consumer.header.msg_length as usize;
        let mut msg = vec![0; msg_size];
        if consumer.pop_body(&mut msg).is_ok() {
            let number = String::from_utf8(msg).unwrap().parse::<i32>().unwrap();
            received_numbers.push(number);
            consumer.commit();
        } else {
            break;
        }
    }

    // Проверяем, что оставшиеся сообщения были получены без пропусков
    check_message_integrity(&received_numbers);
}

use uuid::Uuid;

fn create_unique_queue_path(base_path: &str, prefix: &str) -> String {
    let uuid = Uuid::new_v4().to_string();
    let path = format!("{}/{}_{}", base_path, prefix, uuid);
    fs::remove_dir_all(&base_path).unwrap_or_default();
    path
}

#[test]
fn test_queue_empty() {
    let base_path = create_unique_queue_path("./test-tmp", "queue");
    let queue_name = "test_queue";

    // Создаем очередь без сообщений
    let _queue = Queue::new(&base_path, queue_name, Mode::ReadWrite).unwrap();

    // Создаем потребителя и пытаемся прочитать сообщения
    let consumer_name = "consumer";
    let mut consumer = Consumer::new(&base_path, consumer_name, queue_name).unwrap();

    // Проверяем, что метод pop_header возвращает false, если очередь пуста
    assert!(!consumer.pop_header());
}

#[test]
fn test_multiple_queues() {
    let base_path = create_unique_queue_path("./test-tmp", "queue");
    let queue_name_1 = "test_queue_1";
    let queue_name_2 = "test_queue_2";

    // Создаем две очереди и записываем сообщения в каждую
    let mut queue_1 = Queue::new(&base_path, queue_name_1, Mode::ReadWrite).unwrap();
    let mut queue_2 = Queue::new(&base_path, queue_name_2, Mode::ReadWrite).unwrap();
    let num_messages = 5;
    for i in 0..num_messages {
        let msg = format!("{}", i);
        queue_1.push(msg.as_bytes(), MsgType::String).unwrap();
        queue_2.push(msg.as_bytes(), MsgType::String).unwrap();
    }

    // Создаем потребителей для каждой очереди
    let consumer_name_1 = "consumer_1";
    let consumer_name_2 = "consumer_2";
    let mut consumer_1 = Consumer::new(&base_path, consumer_name_1, queue_name_1).unwrap();
    let mut consumer_2 = Consumer::new(&base_path, consumer_name_2, queue_name_2).unwrap();

    // Читаем сообщения из каждой очереди
    let mut received_numbers_1 = Vec::new();
    let mut received_numbers_2 = Vec::new();
    while consumer_1.pop_header() {
        let msg_size = consumer_1.header.msg_length as usize;
        let mut msg = vec![0; msg_size];
        if consumer_1.pop_body(&mut msg).is_ok() {
            let number = String::from_utf8(msg).unwrap().parse::<i32>().unwrap();
            received_numbers_1.push(number);
            consumer_1.commit();
        } else {
            break;
        }
    }
    while consumer_2.pop_header() {
        let msg_size = consumer_2.header.msg_length as usize;
        let mut msg = vec![0; msg_size];
        if consumer_2.pop_body(&mut msg).is_ok() {
            let number = String::from_utf8(msg).unwrap().parse::<i32>().unwrap();
            received_numbers_2.push(number);
            consumer_2.commit();
        } else {
            break;
        }
    }

    // Проверяем, что сообщения были получены из соответствующих очередей без пропусков
    check_message_integrity(&received_numbers_1);
    check_message_integrity(&received_numbers_2);
}
