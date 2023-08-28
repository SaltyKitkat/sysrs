use std::collections::HashMap;

use rustix::process::{Pid, WaitStatus};
use tokio::sync::mpsc::Receiver;

pub(crate) struct Handler {}
impl Handler {
    fn handle(self, status: WaitStatus) {
        todo!()
    }
}

pub(crate) enum Message {
    Event(Pid, WaitStatus),
    Register(Pid, Handler),
}

pub(crate) struct ProcessMonitor {
    rx: Receiver<Message>,
    map: HashMap<Pid, Handler>,
}

impl ProcessMonitor {
    fn new(rx: Receiver<Message>) -> Self {
        Self {
            rx,
            map: HashMap::new(),
        }
    }

    fn run(mut self) {
        tokio::spawn(async move {
            while let Some(message) = self.rx.recv().await {
                match message {
                    Message::Event(pid, status) => {
                        if let Some(handler) = self.map.remove(&pid) {
                            handler.handle(status)
                        } else {
                            todo!()
                        }
                    }
                    Message::Register(pid, handler) => {
                        self.map.insert(pid, handler);
                    }
                }
            }
        });
    }
}
