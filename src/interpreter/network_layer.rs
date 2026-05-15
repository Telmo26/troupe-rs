use crossbeam::channel::{self, Sender, Receiver};
use dashmap::DashMap;
use uuid::Uuid;

use crate::interpreter::runtime_value::RuntimeValue;

#[derive(Debug, Default)]
pub struct NetworkLayer {
    mailboxes: DashMap<Uuid, (Sender<RuntimeValue>, Receiver<RuntimeValue>)>
}

impl NetworkLayer {
    pub fn send(&self, target: &Uuid, value: RuntimeValue) -> Result<(), NetworkError> {
        let entry = self.mailboxes.get(target).ok_or_else(|| NetworkError::MissingUUID)?;
        let (sender, _) = entry.value();
        sender.send(value).map_err(|_| NetworkError::SendingError)
    }

    pub fn receive(&self, uuid: &Uuid) -> Result<RuntimeValue, NetworkError> {
        let entry = self.mailboxes.get(uuid).ok_or_else(|| NetworkError::MissingUUID)?;
        let (_, receiver) = entry.value();        
        receiver.recv().map_err(|_| NetworkError::ReceiverError)
    }

    pub fn new_local_id(&self, id: &Uuid) {
        let channels = channel::unbounded();
        self.mailboxes.insert(id.clone(), channels);
    }
}

#[derive(Debug)]
pub enum NetworkError {
    MissingUUID,
    SendingError,
    ReceiverError
}