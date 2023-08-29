use std::collections::HashMap;

use rustix::process::{Pid, WaitStatus};
use tokio::sync::mpsc::Receiver;

pub(crate) trait Handler: Send {
    fn handle(self: Box<Self>, pid: Pid, status: WaitStatus);
}

pub(crate) enum Message {
    Event(Pid, WaitStatus),
    Register(Pid, Box<dyn Handler>),
}

pub(crate) struct ProcessMonitor {
    rx: Receiver<Message>,
    map: HashMap<Pid, Box<dyn Handler>>,
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
        });
    }
}
