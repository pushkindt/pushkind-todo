//! Test helper mocking a `ZmqSenderTrait` implementation for services.
use pushkind_common::zmq::{SendFuture, ZmqSenderError, ZmqSenderTrait};

/// Lightweight mock implementation of the [`ZmqSenderTrait`] used in unit tests.
///
/// The mock simply accepts all payloads and resolves immediately which keeps
/// service tests focused on repository behaviour without introducing async
/// coordination overhead.
#[derive(Clone, Debug, Default)]
pub struct MockZmqSender;

impl ZmqSenderTrait for MockZmqSender {
    /// Pretends to asynchronously send a payload and resolves immediately.
    fn send_bytes<'a>(&'a self, bytes: Vec<u8>) -> SendFuture<'a> {
        let _ = bytes;
        Box::pin(async { Ok(()) })
    }

    /// Pretends to synchronously accept payload bytes without error.
    fn try_send_bytes(&self, bytes: Vec<u8>) -> Result<(), ZmqSenderError> {
        let _ = bytes;
        Ok(())
    }

    /// Mocks sending multipart frames and immediately resolves.
    fn send_multipart<'a>(&'a self, frames: Vec<Vec<u8>>) -> SendFuture<'a> {
        let _ = frames;
        Box::pin(async { Ok(()) })
    }
}
