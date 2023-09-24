use std::collections::HashMap;

use tokio::{sync::mpsc::Receiver, task::JoinHandle};

use super::UnitEntry;

pub(crate) struct RtInfo {
    map: HashMap<UnitEntry, JoinHandle<()>>,
}

pub(crate) enum Message {
    Insert(UnitEntry, JoinHandle<()>),
}

impl RtInfo {
    pub(crate) fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub(crate) fn run(mut self, mut rx: Receiver<Message>) -> JoinHandle<()> {
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                match msg {
                    Message::Insert(u, h) => {
                        self.map.insert(u, h);
                    }
                }
            }
        })
    }
}
