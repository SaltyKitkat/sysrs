use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::{io, process::Child, sync::mpsc::Sender};

use super::{
    state::{self, set_state_with_condition, State},
    Unit, UnitCommonImpl, UnitDeps, UnitEntry, UnitImpl, UnitKind,
};
use crate::{unit::state::set_state, Rc};

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

    async fn start(&self, state_manager: Sender<state::Message>) {
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
                    todo!();
                }
                Kind::Forking => todo!(),
                Kind::Oneshot => {
                    tokio::spawn(async move {
                        let exit_state = child.wait().await.unwrap();
                        if exit_state.success() {
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

    async fn stop(&self, state_manager: Sender<state::Message>) {
        todo!()
    }

    async fn restart(&self, state_manager: Sender<state::Message>) {
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
            common: UnitCommonImpl {
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

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Service {
    name: String,
    #[serde(default)]
    requires: String,
    #[serde(default)]
    wants: String,
    #[serde(default)]
    before: String,
    #[serde(default)]
    after: String,
    #[serde(default)]
    conflicts: String,
    kind: Kind,
    start: String,
    stop: String,
    restart: String,
}
impl From<Service> for UnitImpl<Impl> {
    fn from(value: Service) -> Self {
        let Service {
            name,
            requires,
            wants,
            before,
            after,
            conflicts,
            kind,
            start,
            stop,
            restart,
        } = value;
        Self::gen_test(
            name,
            UnitDeps::from_strs(requires, wants, before, after, conflicts),
            kind,
            start,
            stop,
            restart,
        )
    }
}

fn str_to_unitentrys(s: String) -> Box<[UnitEntry]> {
    s.split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| UnitEntry::from(s))
        .collect()
}
impl UnitDeps {
    fn from_strs(
        requires: String,
        wants: String,
        before: String,
        after: String,
        conflicts: String,
    ) -> Self {
        let requires = str_to_unitentrys(requires);
        let wants = str_to_unitentrys(wants);
        let before = str_to_unitentrys(before);
        let after = str_to_unitentrys(after);
        let conflicts = str_to_unitentrys(conflicts);
        Self {
            requires,
            wants,
            after,
            before,
            conflicts,
        }
    }
}
