use rustix::{process::WaitStatus, thread::Pid};
use tokio::{
    sync::mpsc::{self, Receiver, Sender},
    task::JoinHandle,
};

mod process;

pub(crate) struct Monitor {
    process: Sender<process::Message>,
}

pub(crate) enum Message {
    Process((Pid, WaitStatus)),
}
impl Monitor {
    pub(crate) fn new() -> Self {
        let (process, process_rx) = mpsc::channel(4);
        process::Monitor::new().run(process_rx);
        Self { process }
    }

    pub(crate) fn run(mut self, mut rx: Receiver<Message>) -> JoinHandle<()> {
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                self.serve(msg);
            }
        })
    }

    fn serve(&mut self, msg: Message) {
        match msg {
            Message::Process((pid, status)) => {
                let process = self.process.clone();
                // spawn a task to send msg
                // prevent loop msg send lead to dead lock
                tokio::spawn(async move {
                    process
                        .send(process::Message::Event(pid, status))
                        .await
                        .unwrap()
                });
            }
        }
    }
}
