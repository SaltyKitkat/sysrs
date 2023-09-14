use std::fmt::{Debug, Display};

use tokio::sync::mpsc::Sender;

use crate::{util::job, Rc};

pub(crate) mod mount;
pub(crate) mod service;
pub(crate) mod state;
pub(crate) mod store;

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum UnitKind {
    Service,
    Timer,
    Mount,
    Target,
    Socket,
}

impl Display for UnitKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            UnitKind::Service => "service",
            UnitKind::Timer => "timer",
            UnitKind::Mount => "mount",
            UnitKind::Target => "target",
            UnitKind::Socket => "socket",
        };
        f.write_str(s)
    }
}

#[derive(Debug)]
pub(crate) struct UnitCommonImpl {
    name: Rc<str>,
    description: Rc<str>,
    documentation: Rc<str>,
    deps: Rc<UnitDeps>, // todo
}

#[derive(Debug, Default)]
pub(crate) struct UnitDeps {
    requires: Box<[UnitEntry]>,
    wants: Box<[UnitEntry]>,
    after: Box<[UnitEntry]>,
    before: Box<[UnitEntry]>,
    conflicts: Box<[UnitEntry]>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub(crate) struct UnitEntry {
    name: Rc<str>,
}

impl From<&str> for UnitEntry {
    fn from(value: &str) -> Self {
        Self { name: value.into() }
    }
}

pub(crate) trait Unit: Debug {
    fn name(&self) -> Rc<str>;
    fn description(&self) -> Rc<str>;
    fn documentation(&self) -> Rc<str>;
    fn kind(&self) -> UnitKind;

    fn deps(&self) -> Rc<UnitDeps>;

    fn start(&self, job_manager: Sender<job::Message>);
    fn stop(&self, job_manager: Sender<job::Message>);
    fn restart(&self, job_manager: Sender<job::Message>);
}

#[derive(Debug)]
pub(crate) struct UnitImpl<KindImpl> {
    pub common: UnitCommonImpl,
    pub sub: KindImpl,
}

impl<T: Unit> From<&T> for UnitEntry {
    fn from(value: &T) -> Self {
        Self {
            name: value.name().clone(),
        }
    }
}
