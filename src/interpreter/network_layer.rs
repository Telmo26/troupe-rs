use crossbeam::channel::{self, Sender, Receiver};
use dashmap::DashMap;
use uuid::Uuid;

use crate::parser::AST;

#[derive(Debug, Default)]
pub struct NetworkLayer {
    mailboxes: DashMap<Uuid, (Sender<AST>, Receiver<AST>)>
}

impl NetworkLayer {
    pub fn send(&self, target: &Uuid, value: AST) -> Result<(), NetworkError> {
        let entry = self.mailboxes.get(target).ok_or_else(|| NetworkError::MissingUUID)?;
        let (sender, _) = entry.value();
        sender.send(value).map_err(|_| NetworkError::SendingError)
    }

    pub fn receive(&self, uuid: &Uuid) -> Result<AST, NetworkError> {
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