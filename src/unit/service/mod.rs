use std::process::ExitStatus;

use futures::Future;
use tokio::{io, process};

use crate::Rc;

use super::{Unit, UnitDeps, UnitImpl, UnitKind};

#[derive(Debug)]
pub struct ServiceImpl {
    exec_start: Rc<str>,
    exec_stop: Rc<str>,
    exec_restart: Rc<str>,
}

impl ServiceImpl {
    pub fn new(start: Rc<str>, stop: Rc<str>, restart: Rc<str>) -> Self {
        Self {
            exec_start: start,
            exec_stop: stop,
            exec_restart: restart,
        }
    }
}

impl Unit for UnitImpl<ServiceImpl> {
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
