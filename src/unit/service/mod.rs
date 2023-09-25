use async_trait::async_trait;
use tokio::{io, process::Child, select, sync::mpsc::Sender};

use super::{
    guard::{self, create_guard, guard_stop},
    state::{self, set_state_with_condition, State},
    Unit, UnitCommon, UnitDeps, UnitEntry, UnitImpl, UnitKind,
};
use crate::{unit::state::set_state, Rc};

pub(crate) mod loader;

#[derive(Clone, Copy, Debug, serde::Deserialize, serde::Serialize)]
pub(crate) enum Kind {
    Simple,
    Forking,
    Oneshot,
    Notify,
}

#[derive(Debug)]
pub(crate) struct Impl {
    kind: Kind,
    exec_start: Rc<str>,
    exec_stop: Rc<str>,
    exec_restart: Rc<str>,
}

impl Impl {
    pub fn new(kind: Kind, start: Rc<str>, stop: Rc<str>, restart: Rc<str>) -> Self {
        Self {
            kind,
            exec_start: start,
            exec_stop: stop,
            exec_restart: restart,
        }
    }
}

#[async_trait]
impl Unit for UnitImpl<Impl> {
    fn name(&self) -> Rc<str> {
        Rc::clone(&self.common.name)
    }

    fn description(&self) -> Rc<str> {
        Rc::clone(&self.common.description)
    }

    fn documentation(&self) -> Rc<str> {
        Rc::clone(&self.common.documentation)
    }

    fn kind(&self) -> UnitKind {
        UnitKind::Service
    }

    fn deps(&self) -> Rc<UnitDeps> {
        self.common.deps.clone()
    }

    async fn start(
        &self,
        state_manager: Sender<state::Message>,
        guard_manager: Sender<guard::Message>,
    ) {
        // todo: send job to job manager and let it to set state due to job status
        // unit status is expected to be starting here
        let kind = self.sub.kind;
        let entry = UnitEntry::from(self);
        match set_state_with_condition(&state_manager, entry.clone(), State::Starting, |s| {
            s.is_inactive()
        })
        .await
        {
            Ok(_) => (),
            Err(_) => todo!(),
        }
        match run_cmd(&self.sub.exec_start) {
            Ok(mut child) => match kind {
                Kind::Simple => {
                    set_state(&state_manager, entry.clone(), State::Active).await;
                    println!("simple service: state set!");
                    create_guard(&guard_manager, entry.clone(), |store, mut rx| async move {
                        select! {
                            exit_status = child.wait() => {
                                let exit_status = exit_status.unwrap();
                                if exit_status.success() {
                                    State::Stopped
                                } else {
                                    State::Failed
                                }
                            },
                            msg = rx.recv() => {
                                match msg.unwrap() {
                                    guard::GMessage::Stop => todo!(),
                                    guard::GMessage::Kill => child.kill().await.unwrap(),
                                }
                                State::Stopped
                            }
                        }
                    })
                    .await;
                }
                Kind::Forking => todo!(),
                Kind::Oneshot => {
                    tokio::spawn(async move {
                        let exit_status = child.wait().await.unwrap();
                        if exit_status.success() {
                            set_state(&state_manager, entry, State::Stopped).await
                        } else {
                            set_state(&state_manager, entry, State::Failed).await
                        }
                    });
                }
                Kind::Notify => todo!(),
            },
            Err(_) => {
                set_state_with_condition(&state_manager, entry, State::Failed, |s| {
                    s == State::Starting
                })
                .await
                .expect("previous state should be `Starting`");
            }
        }
    }

    async fn stop(
        &self,
        state_manager: Sender<state::Message>,
        guard_manager: Sender<guard::Message>,
    ) {
        let entry = UnitEntry::from(self);
        match set_state_with_condition(&state_manager, entry.clone(), State::Stopping, |s| {
            s.is_active()
        })
        .await
        {
            Ok(_) => (),
            Err(_) => todo!(),
        }

        match self.sub.kind {
            Kind::Simple => {
                guard_stop(&guard_manager, entry).await;
            }
            Kind::Forking => todo!(),
            Kind::Oneshot => {
                let child = run_cmd(&self.sub.exec_stop).unwrap().wait().await;
            }
            Kind::Notify => todo!(),
        }
    }

    async fn restart(
        &self,
        state_manager: Sender<state::Message>,
        guard_manager: Sender<guard::Message>,
    ) {
        todo!()
    }
}

fn run_cmd(exec_start: &str) -> Result<Child, io::Error> {
    let mut s = exec_start.split_whitespace();
    tokio::process::Command::new(s.next().unwrap())
        .args(s)
        .spawn()
}

impl UnitImpl<Impl> {
    fn gen_test(
        name: impl AsRef<str>,
        deps: UnitDeps,
        kind: Kind,
        start: impl AsRef<str>,
        stop: impl AsRef<str>,
        restart: impl AsRef<str>,
    ) -> Self {
        let null_str: Rc<str> = "".into();
        Self {
            common: UnitCommon {
                name: name.as_ref().into(),
                description: null_str.clone(),
                documentation: null_str.clone(),
                deps: Rc::new(deps),
            },
            sub: Impl {
                kind,
                exec_start: start.as_ref().into(),
                exec_stop: start.as_ref().into(),
                exec_restart: start.as_ref().into(),
            },
        }
    }
}
