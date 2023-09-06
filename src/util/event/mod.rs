use rustix::process::{Pid, WaitStatus};
use tokio::sync::mpsc::{Receiver, Sender};

pub(crate) mod signal;

pub(crate) enum Event {
    SigChld(Pid, WaitStatus),
}

pub(crate) struct EventHandler {
    rx: Receiver<Event>,
    monitor: Sender<(Pid, WaitStatus)>,
}

impl EventHandler {
    pub(crate) fn new(rx: Receiver<Event>, monitor: Sender<(Pid, WaitStatus)>) -> Self {
        Self { rx, monitor }
    }

    pub(crate) fn run(mut self) {
        tokio::spawn(async move {
            while let Some(event) = self.rx.recv().await {
                match event {
                    Event::SigChld(pid, status) => {
                        self.monitor
                            .send((pid, status))
                            .await
                            .expect("monitor channel closed unexpectly!");
                    }
                }
            }
        });
    }
}
