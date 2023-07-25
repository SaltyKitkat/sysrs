use std::fmt::{Debug, Display};

use crate::Rc;

pub(crate) mod mount;
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
pub struct UnitCommonImpl {
    name: Rc<str>,
    description: Rc<str>,
    documentation: Rc<str>,
    deps: UnitDeps, // todo
}

#[derive(Default, Debug)]
pub struct UnitDeps {
    requires: Vec<UnitEntry>,
    required_by: Vec<UnitEntry>,
    // wants: (),
    // after: (),
    // before: (),
    // confilcts: (),
}

#[derive(Clone, Copy, Debug)]
pub enum UnitStatus {
    Uninit,
    Stopped,
    Starting,
    Running,
    Stopping,
}
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct UnitEntry {
    name: Rc<str>,
}

impl From<&str> for UnitEntry {
    fn from(value: &str) -> Self {
        Self { name: value.into() }
    }
}

pub trait Unit {
    fn name(&self) -> Rc<str>;
    fn description(&self) -> Rc<str>;
    fn documentation(&self) -> Rc<str>;
    fn kind(&self) -> UnitKind;

    fn deps(&self) -> UnitDeps;

    fn start(&mut self);
    fn stop(&mut self);
    fn restart(&mut self);
}

#[derive(Debug)]
pub struct UnitImpl<KindImpl> {
    common: UnitCommonImpl,
    kind: KindImpl,
}

impl From<&dyn Unit> for UnitEntry {
    fn from(value: &dyn Unit) -> Self {
        Self {
            name: value.name().clone(),
        }
    }
}
