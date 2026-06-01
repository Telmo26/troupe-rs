use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use dashmap::DashMap;
use uuid::Uuid;

use crate::interpreter::runtime_value::RuntimeValue;

#[derive(Debug, Default)]
pub struct NetworkLayer {
    mailboxes: DashMap<Uuid, UnboundedSender<RuntimeValue>>
}

impl NetworkLayer {
    pub async fn send(&self, target: &Uuid, value: RuntimeValue) -> Result<(), NetworkError> {
        let entry = self.mailboxes.get(target).ok_or_else(|| NetworkError::MissingUUID)?;
        let sender = entry.value();
        sender.send(value).map_err(|_| NetworkError::SendingError)
    }

    pub fn new_local_id(&self, id: Uuid) -> UnboundedReceiver<RuntimeValue> {
        let (sender, receiver) = mpsc::unbounded_channel();
        self.mailboxes.insert(id, sender);
        receiver
    }
}

#[derive(Debug)]
pub enum NetworkError {
    MissingUUID,
    SendingError
}