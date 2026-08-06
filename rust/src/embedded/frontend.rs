use async_trait::async_trait;
use intiface_engine::{EngineMessage, Frontend, IntifaceError, IntifaceMessage};
use std::sync::Arc;
use tokio::sync::{Notify, broadcast};

/// A frontend for the engine that does not bridge to a Dart stream.
///
/// Mirrors [`crate::in_process_frontend::FlutterIntifaceEngineFrontend`] but
/// drops messages instead of pushing them through a `StreamSink`.
pub struct EmbeddedIntifaceEngineFrontend {
  sender: Arc<broadcast::Sender<IntifaceMessage>>,
  disconnect_notifier: Arc<Notify>,
}

impl EmbeddedIntifaceEngineFrontend {
  pub fn new(sender: Arc<broadcast::Sender<IntifaceMessage>>) -> Self {
    Self {
      sender,
      disconnect_notifier: Arc::new(Notify::new()),
    }
  }
}

#[async_trait]
impl Frontend for EmbeddedIntifaceEngineFrontend {
  async fn connect(&self) -> Result<(), IntifaceError> {
    Ok(())
  }

  fn disconnect(&self) {
    self.disconnect_notifier.notify_waiters();
  }

  fn disconnect_notifier(&self) -> Arc<Notify> {
    self.disconnect_notifier.clone()
  }

  fn event_stream(&self) -> broadcast::Receiver<IntifaceMessage> {
    self.sender.subscribe()
  }

  async fn send(&self, msg: EngineMessage) {
    trace!("Engine message: {msg:?}");
  }
}
