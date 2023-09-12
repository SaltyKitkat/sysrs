use tokio::sync::mpsc::Sender;

use super::{Unit, UnitDeps, UnitImpl, UnitKind};
use crate::{util::job, Rc};

#[derive(Clone, Copy, Debug)]
pub enum Kind {
    Simple,
    Forking,
    Oneshot,
    Notify,
}

#[derive(Debug)]
pub struct Impl {
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

    fn deps(&self) -> UnitDeps {
        todo!()
    }

    fn start(&self, job_manager: Sender<job::Message>) {
        // todo: check state and set state: starting (CAS)
        // should impl in UnitStore, not in Unit::start
        // todo: send job to job manager and let it to set state due to job status
    }

    fn stop(&self, job_manager: Sender<job::Message>) {
        todo!()
    }

    fn restart(&self, job_manager: Sender<job::Message>) {
        todo!()
    }
}
