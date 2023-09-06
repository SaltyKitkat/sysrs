use tokio::{sync::mpsc::Receiver, task::JoinHandle};

mod process;

pub(crate) struct Monitor {}

pub(crate) struct Message {}
impl Monitor {
    pub(crate) fn new() -> Self {
        Self {}
    }

    pub(crate) fn run(mut self, mut rx: Receiver<Message>) -> JoinHandle<()> {
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                todo!()
            }
        })
    }
}
