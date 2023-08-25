use std::collections::HashMap;

use futures::select_biased;
use rustix::process::{Pid, WaitStatus};
use tokio::sync::mpsc::Receiver;

pub(crate) enum Message {
    Event(Pid, WaitStatus),
    Register(Pid, ()),
}

pub(crate) struct Monitor {
    rx: Receiver<Message>,
    map: HashMap<Pid, ()>,
}

impl Monitor {
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
                    Message::Event(_, _) => todo!(),
                    Message::Register(_, _) => todo!(),
                }
            }
        });
    }
}
