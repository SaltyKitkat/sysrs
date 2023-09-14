use futures::{future::BoxFuture, Future};
use tokio::{
    process::{Child, Command},
    sync::mpsc::{Receiver, Sender},
    task::JoinHandle,
};

use crate::{
    unit::{
        state::{self, set_state, State},
        store, UnitEntry,
    },
    Rc,
};

use super::monitor;

pub(crate) struct JobManager {
    state: Sender<state::Message>,
    monitor: Sender<monitor::Message>,
}

pub(crate) enum Job {
    RunCmd {
        cmd: Rc<str>,
        handler: Box<dyn FnOnce(Child) -> State + Send>,
    },
    RunBlocking(Box<dyn FnOnce() -> State + Send>),
    RunAsync(BoxFuture<'static, State>),
}

pub(crate) struct Message {
    unit: UnitEntry,
    job: Job,
}

impl JobManager {
    pub(crate) fn new(
        store: Sender<store::Message>,
        state: Sender<state::Message>,
        monitor: Sender<monitor::Message>,
    ) -> Self {
        Self { state, monitor }
    }

    pub(crate) fn run(self, mut rx: Receiver<Message>) -> JoinHandle<()> {
        tokio::spawn(async move {
            while let Some(Message { unit, job }) = rx.recv().await {
                // do the job, and set the unit state due to job result
                let state_manager = self.state.clone();
                tokio::spawn(async move {
                    let new_state = match job {
                        Job::RunCmd { cmd, handler } => {
                            let child = run_cmd(cmd.as_ref()).unwrap();
                            handler(child)
                        }
                        Job::RunBlocking(f) => f(),
                        Job::RunAsync(f) => f.await,
                    };
                    set_state(&state_manager, unit, new_state).await
                });
            }
        })
    }
}

fn run_cmd(cmd: &str) -> std::io::Result<Child> {
    let mut cmd = cmd.split_ascii_whitespace();
    Command::new(cmd.next().unwrap()).args(cmd).spawn()
}

pub(crate) async fn create_cmd_job(
    job_manager: &Sender<Message>,
    unit: UnitEntry,
    cmd: Rc<str>,
    handler: impl FnOnce(Child) -> State + Send + 'static,
) {
    fn to_boxed(h: impl FnOnce(Child) + Send + 'static) -> Box<dyn FnOnce(Child) + Send + 'static> {
        Box::new(h)
    }
    job_manager
        .send(Message {
            unit,
            job: Job::RunCmd {
                cmd,
                handler: Box::new(handler),
            },
        })
        .await
        .unwrap();
}

pub(crate) async fn create_async_job(
    job_manager: &Sender<Message>,
    unit: UnitEntry,
    job: impl Future<Output = State> + Send + 'static,
) {
    job_manager
        .send(Message {
            unit,
            job: Job::RunAsync(Box::pin(job)),
        })
        .await
        .unwrap();
}

pub(crate) async fn create_blocking_job(
    job_manager: &Sender<Message>,
    unit: UnitEntry,
    job: impl FnOnce() -> State + Send + 'static,
) {
    job_manager
        .send(Message {
            unit,
            job: Job::RunBlocking(Box::new(job)),
        })
        .await
        .unwrap();
}
