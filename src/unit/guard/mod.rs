use std::collections::HashMap;

use futures::Future;
use tokio::{
    sync::mpsc::{Receiver, Sender},
    task::JoinHandle,
};

use super::{state, store, UnitEntry};

#[derive(Debug, Clone)]
pub(crate) struct GuardManager {
    map: HashMap<UnitEntry, Sender<GMessage>>,
    store: Sender<store::Message>,
    state: Sender<state::Message>,
}

pub(crate) enum Message {
    Update(UnitEntry, Sender<GMessage>),
    Remove(UnitEntry),
    Stop(UnitEntry),
    Kill(UnitEntry),
}

impl GuardManager {
    pub(crate) fn new() -> Self {
        todo!()
    }

    pub(crate) fn run(mut self, mut rx: Receiver<Message>) -> JoinHandle<()> {
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                match msg {
                    Message::Update(u, s) => {
                        self.map.insert(u, s);
                    }
                    Message::Remove(u) => {
                        self.map.remove(&u);
                    }
                    Message::Stop(u) => {
                        self.map
                            .get(&u)
                            .unwrap()
                            .send(GMessage::Stop)
                            .await
                            .unwrap();
                    }
                    Message::Kill(u) => self
                        .map
                        .get(&u)
                        .unwrap()
                        .send(GMessage::Kill)
                        .await
                        .unwrap(),
                }
            }
        })
    }
}

pub(crate) enum GMessage {
    Stop,
    Kill,
}
