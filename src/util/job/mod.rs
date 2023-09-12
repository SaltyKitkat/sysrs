use futures::{future::BoxFuture, Future};
use tokio::{
    process::{Child, Command},
    sync::mpsc::{Receiver, Sender},
    task::JoinHandle,
};

use crate::{unit::state, Rc};

use super::monitor;

pub(crate) struct JobManager {
    state: Sender<state::Message>,
    monitor: Sender<monitor::Message>,
}

pub(crate) enum Message {
    RunCmd {
        cmd: Rc<str>,
        // handler: tokio::spawn(async move {child.await; /* send msg to some actor */})
        handler: Option<Box<dyn FnOnce(Child) + Send>>,
    },
    RunBlocking(Box<dyn FnOnce() + Send>),
    RunAsync(BoxFuture<'static, ()>),
}

impl JobManager {
    pub(crate) fn new(state: Sender<state::Message>, monitor: Sender<monitor::Message>) -> Self {
        Self { state, monitor }
    }

    pub(crate) fn run(self, mut rx: Receiver<Message>) -> JoinHandle<()> {
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                match msg {
                    Message::RunCmd { cmd, handler } => {
                        let child = run_cmd(cmd.as_ref()).unwrap();
                        handler.map(|h| h(child));
                    }
                    Message::RunBlocking(f) => {
                        tokio::task::spawn_blocking(f); // todo: handle task failure
                    }
                    Message::RunAsync(f) => {
                        tokio::spawn(f); // todo: handle task failure
                    }
                }
            }
        })
    }
}

fn run_cmd(cmd: &str) -> std::io::Result<Child> {
    let mut cmd = cmd.split_ascii_whitespace();
    Command::new(cmd.next().unwrap()).args(cmd).spawn()
}

pub(crate) fn create_cmd_job(
    job_manager: Sender<Message>,
    cmd: Rc<str>,
    handler: Option<impl FnOnce(Child) + Send + 'static>,
) {
    fn to_boxed(h: impl FnOnce(Child) + Send + 'static) -> Box<dyn FnOnce(Child) + Send + 'static> {
        Box::new(h)
    }
    tokio::spawn(async move {
        job_manager
            .send(Message::RunCmd {
                cmd,
                handler: handler.map(to_boxed),
            })
            .await
    });
}

pub(crate) fn create_async_job(
    job_manager: Sender<Message>,
    job: impl Future<Output = ()> + Send + 'static,
) {
    tokio::spawn(async move { job_manager.send(Message::RunAsync(Box::pin(job))).await });
}

pub(crate) fn create_blocking_job(
    job_manager: Sender<Message>,
    job: impl FnOnce() + Send + 'static,
) {
    tokio::spawn(async move { job_manager.send(Message::RunBlocking(Box::new(job))).await });
}
