use std::fmt::{Debug, Display};

use async_trait::async_trait;

use crate::Rc;

use self::state::State;

pub(crate) mod dep;
pub(crate) mod guard;
pub(crate) mod mount;
pub(crate) mod service;
pub(crate) mod socket;
pub(crate) mod state;
pub(crate) mod store;
pub(crate) mod target;

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub(crate) enum UnitKind {
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
pub(crate) struct UnitCommon {
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
impl Display for UnitEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl From<&str> for UnitEntry {
    fn from(value: &str) -> Self {
        Self { name: value.into() }
    }
}

impl<T: Unit + ?Sized> From<&T> for UnitEntry {
    fn from(value: &T) -> Self {
        Self { name: value.name() }
    }
}

#[async_trait]
pub(crate) trait Handle: Send {
    /// use runtime info to stop the running things
    async fn stop(self: Box<Self>) -> Result<(), UnitHandle>;

    /// monitor runtime state, and return exit state when it's dead
    async fn wait(&mut self) -> State;
}
type UnitHandle = Box<dyn Handle>;

#[async_trait]
pub(crate) trait Unit: Debug {
    fn name(&self) -> Rc<str>;
    fn description(&self) -> Rc<str>;
    fn documentation(&self) -> Rc<str>;
    fn kind(&self) -> UnitKind;

    fn deps(&self) -> Rc<UnitDeps>;

    /// start the unit, return a handle which
    /// contains runtime info needed for monitor and stop/kill
    async fn start(&self) -> Result<UnitHandle, ()>; // todo: error type

    /// do things needed to stop the unit
    async fn stop(&self, handle: UnitHandle) -> Result<(), ()>;

    async fn restart(&self, handle: UnitHandle) -> Result<UnitHandle, ()>;
}

pub(crate) type UnitObj = Rc<dyn Unit + Send + Sync + 'static>;

#[derive(Debug)]
pub(crate) struct UnitImpl<KindImpl> {
    pub common: UnitCommon,
    pub sub: KindImpl,
}
