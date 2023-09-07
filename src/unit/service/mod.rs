use std::process::ExitStatus;

use futures::Future;
use rustix::thread::Pid;
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
        // todo: set state: starting
        match run_cmd(&self.sub.exec_start) {
            Ok(pid) => {
                let handler = match self.sub.kind {
                    Kind::Simple => {
                        // todo: handle exit code and set state
                    }
                    Kind::Forking => todo!(),
                    Kind::Oneshot => todo!(),
                    Kind::Notify => todo!(),
                };
            }
            Err(e) => todo!("handle error"),
        };
        // todo: set state: running
    }

    fn stop(&mut self) {
        match run_cmd(&self.sub.exec_stop) {
            Ok(_) => todo!(),
            Err(_) => todo!(),
        };
    }

    fn restart(&mut self) {
        match run_cmd(&self.sub.exec_restart) {
            Ok(_) => todo!(),
            Err(_) => todo!(),
        };
    }
}

pub(self) fn run_cmd(cmd: &str) -> std::io::Result<Option<Pid>> {
    let mut cmd = cmd.split_ascii_whitespace();
    Ok(process::Command::new(cmd.next().unwrap())
        .args(cmd)
        .spawn()?
        .id()
        .and_then(|pid| Pid::from_raw(pid as _)))
}
