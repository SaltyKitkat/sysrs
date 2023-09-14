use tokio::sync::mpsc::Sender;

use super::{Unit, UnitDeps, UnitImpl, UnitKind};
use crate::{util::job, Rc};

#[derive(Clone, Copy, Debug)]
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

    fn start(&self, job_manager: Sender<job::Message>) {
        // todo: send job to job manager and let it to set state due to job status
        match self.sub.kind {
            Kind::Simple => todo!(),
            Kind::Forking => todo!(),
            Kind::Oneshot => todo!(),
            Kind::Notify => todo!(),
        }
    }

    fn stop(&self, job_manager: Sender<job::Message>) {
        todo!()
    }

    fn restart(&self, job_manager: Sender<job::Message>) {
        todo!()
    }
}
