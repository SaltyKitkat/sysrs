use std::collections::HashMap;

use rustix::process::{Pid, WaitStatus};
use tokio::{sync::mpsc::Receiver, task::JoinHandle};

pub(crate) trait Handler: Send {
    fn handle(self: Box<Self>, pid: Pid, status: WaitStatus);
}

pub(crate) enum Message {
    Event(Pid, WaitStatus),
    Register(Pid, Box<dyn Handler>),
}

pub(crate) struct Monitor {
    map: HashMap<Pid, Box<dyn Handler>>,
}

impl Monitor {
    pub(super) fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub(super) fn run(mut self, mut rx: Receiver<Message>) -> JoinHandle<()> {
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                match msg {
                    Message::Event(pid, status) => {
                        if let Some(handler) = self.map.remove(&pid) {
                            handler.handle(pid, status)
                        } else {
                            todo!()
                        }
                    }
                    Message::Register(pid, handler) => {
                        self.map.insert(pid, handler);
                    }
                }
            }
        })
    }
}
