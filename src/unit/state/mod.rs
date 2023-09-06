use std::collections::HashMap;

use tokio::{sync::mpsc::Receiver, task::JoinHandle};

use super::UnitEntry;

#[derive(Clone, Copy, Debug)]
pub enum State {
    Uninit,
    Stopped,
    Failed,
    Starting,
    Running,
    Stopping,
}

pub(crate) struct StateMap {
    map: HashMap<UnitEntry, State>,
}
pub(crate) enum Action {
    Set(State),
}
pub(crate) struct Message(UnitEntry, Action);

impl StateMap {
    pub(crate) fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub(crate) fn run(mut self, mut rx: Receiver<Message>) -> JoinHandle<()> {
        tokio::task::spawn(async move {
            while let Some(msg) = rx.recv().await {
                let entry = msg.0;
                match msg.1 {
                    Action::Set(new_state) => {
                        if let Some(state) = self.map.get_mut(&entry) {
                            *state = new_state;
                        } else {
                            todo!("handle missing unit")
                        }
                    }
                }
            }
        })
    }
}
