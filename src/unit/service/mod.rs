use tokio::process;

use crate::Rc;

use super::{Unit, UnitCommonImpl, UnitDeps, UnitImpl, UnitKind};

mod sshd;

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
        let exec_start = &self.kind.exec_start;
        let mut args = exec_start.split_ascii_whitespace();
        let child = process::Command::new(args.next().unwrap())
            .args(args)
            .spawn();
    }

    fn stop(&mut self) {
        let exec_stop = &self.kind.exec_stop;
        let mut args = exec_stop.split_ascii_whitespace();
        let status = process::Command::new(args.next().unwrap())
            .args(args)
            .status();
    }

    fn restart(&mut self) {
        todo!()
    }
}
