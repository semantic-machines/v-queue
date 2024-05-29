# Documentation for the v_queue Library

v_queue is a simple file-based queue library for Rust that follows the "one writer, many readers" principle. It provides functionality for working with message queues, allowing a single writer to write messages to a queue and multiple readers to read messages from the queue concurrently.

## Introduction

The v_queue library provides a straightforward way to create and manage message queues using a file-based storage system. It allows creating queues, writing messages to them, and reading messages from queues using consumers. The library is designed to be simple and efficient, making it suitable for various use cases where reliable message queuing is required.

## Creating a Queue

To create a new queue, the `Queue` structure is used. Here's an example of creating a queue:

```rust
use v_queue::queue::Queue;
use v_queue::record::Mode;

let base_path = "./queue_data";
let queue_name = "my_queue";
let mode = Mode::ReadWrite;

let mut queue = Queue::new(base_path, queue_name, mode).unwrap();
```

- `base_path` - the path to the directory where the queue data will be stored.
- `queue_name` - the name of the queue.
- `mode` - the access mode for the queue. It can be `Mode::ReadWrite` for reading and writing or `Mode::Read` for read-only access.

## Writing Messages to a Queue

To write a message to a queue, the `push` method of the `Queue` structure is used. Here's an example of writing a message:

```rust
use v_queue::record::MsgType;

let message = "Hello, world!".as_bytes();
let msg_type = MsgType::String;

queue.push(message, msg_type).unwrap();
```

- `message` - the content of the message as a byte slice.
- `msg_type` - the type of the message. It can be `MsgType::String` for text messages or `MsgType::Object` for binary data.

## Creating a Consumer

To read messages from a queue, the `Consumer` structure is used. Here's an example of creating a consumer:

```rust
use v_queue::consumer::Consumer;

let consumer_name = "my_consumer";

let mut consumer = Consumer::new(base_path, consumer_name, queue_name).unwrap();
```

- `base_path` - the path to the directory where the queue data is stored.
- `consumer_name` - the name of the consumer.
- `queue_name` - the name of the queue from which messages will be read.

## Reading Messages from a Queue

To read messages from a queue, the methods of the `Consumer` structure are used. Here's an example of reading messages:

```rust
while consumer.pop_header() {
    let msg_size = consumer.header.msg_length as usize;
    let mut msg = vec![0; msg_size];
    if consumer.pop_body(&mut msg).is_ok() {
        // Process the message
        let message = String::from_utf8(msg).unwrap();
        println!("Received message: {}", message);
        consumer.commit();
    } else {
        break;
    }
}
```

- `pop_header` - retrieves the header of the next message. Returns `true` if the header is successfully obtained and `false` if the queue is empty.
- `header.msg_length` - the length of the message in bytes.
- `pop_body` - retrieves the body of the message and writes it to the provided `msg` buffer.
- `commit` - confirms the processing of the message and removes it from the queue.

## ReadOnly Mode

A queue can be created in `Mode::Read` mode, which allows only reading messages from the queue without the ability to write new messages. This can be useful in scenarios where data immutability in the queue needs to be ensured.

Example of creating a queue in `ReadOnly` mode:

```rust
use v_queue::queue::Queue;
use v_queue::record::Mode;

let base_path = "./queue_data";
let queue_name = "my_queue";
let mode = Mode::Read;

let queue = Queue::new(base_path, queue_name, mode).unwrap();
```

In `ReadOnly` mode, attempting to write a message to the queue using the `push` method will result in an `ErrorQueue::NotReady` error.

## Error Handling

Various errors can occur when working with queues and consumers, represented by the `ErrorQueue` enumeration. Here are the possible error variants:

- `ErrorQueue::NotReady`: The queue is not ready.
- `ErrorQueue::AlreadyOpen`: The queue is already open.
- `ErrorQueue::FailWrite`: Write failure.
- `ErrorQueue::InvalidChecksum`: Invalid checksum.
- `ErrorQueue::FailReadTailMessage`: Failed to read the tail message.
- `ErrorQueue::FailOpen`: Open failure.
- `ErrorQueue::FailRead`: Read failure.
- `ErrorQueue::NotFound`: Not found.
- `ErrorQueue::Other`: Other error.

When an error occurs, the library methods return a `Result<T, ErrorQueue>` value, where `T` is the expected result in case of success, and `ErrorQueue` is the possible error. You can handle these errors using the `unwrap()`, `expect()`, or `match` pattern matching.

## Conclusion

The v_queue library provides a simple and efficient way to work with message queues in Rust. It follows the "one writer, many readers" principle, allowing a single writer to write messages to a queue and multiple readers to read messages concurrently. The library's file-based storage system ensures data persistence and enables easy integration into various applications. With its straightforward API and support for read-only mode, v_queue offers flexibility and reliability for message queuing scenarios.
