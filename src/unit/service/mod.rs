use std::process::ExitStatus;

use futures::Future;
use tokio::{io, process};

use super::{Unit, UnitDeps, UnitImpl, UnitKind};
use crate::Rc;

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

    fn start(&mut self) {
        tokio::spawn(run_cmd(&self.kind.exec_start));
    }

    fn stop(&mut self) {
        tokio::spawn(run_cmd(&self.kind.exec_stop));
    }

    fn restart(&mut self) {
        tokio::spawn(run_cmd(&self.kind.exec_restart));
    }
}

pub(self) fn run_cmd(cmd: &str) -> impl Future<Output = io::Result<ExitStatus>> {
    let mut cmd = cmd.split_ascii_whitespace();
    process::Command::new(cmd.next().unwrap())
        .args(cmd)
        .status()
}
