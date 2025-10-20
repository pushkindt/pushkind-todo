use pushkind_common::zmq::{ZmqSenderExt, ZmqSenderTrait};

pub struct MockZmqSender {}

impl ZmqSenderTrait for MockZmqSender {}

impl ZmqSenderExt for MockZmqSender {
    fn send_bytes<'a>(&'a self, bytes: Vec<u8>) -> SendFuture<'a> {}

    /// Try to send raw bytes (fails fast if the queue is full).
    fn try_send_bytes(&self, bytes: Vec<u8>) -> Result<(), ZmqSenderError> {}

    /// Send multipart frames (awaits if the queue is full).
    fn send_multipart<'a>(&'a self, frames: Vec<Vec<u8>>) -> SendFuture<'a> {}
}
