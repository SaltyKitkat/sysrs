use std::collections::HashMap;

use tap::Pipe;
use tokio::{
    sync::{mpsc::Receiver, oneshot},
    task::JoinHandle,
};

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
    Get(oneshot::Sender<Option<State>>),
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
                        self.map.insert(entry, new_state);
                    }
                    Action::Get(s) => self.map.get(&entry).copied().pipe(|state| {
                        s.send(state).ok();
                    }),
                }
            }
        })
    }
}
