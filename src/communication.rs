pub mod bound;
pub mod unbound;

pub struct CommunicationHandle<In = ()> {
    pub(crate) _channel_tx: async_channel::Sender<In>,
}

impl<In> Clone for CommunicationHandle<In> {
    fn clone(&self) -> Self {
        Self {
            _channel_tx: self._channel_tx.clone(),
        }
    }
}

impl<In> CommunicationHandle<In> {
    pub(crate) fn new(channel_tx: async_channel::Sender<In>) -> Self {
        Self {
            _channel_tx: channel_tx,
        }
    }

    pub async fn send(&self, data: In) -> std::io::Result<()> {
        let _ = self._channel_tx.send(data).await;
        Ok(())
    }
}
