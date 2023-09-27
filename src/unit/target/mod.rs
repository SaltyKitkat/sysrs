use async_trait::async_trait;
use tokio::sync::mpsc::Sender;

use super::{guard, state, Unit, UnitDeps, UnitImpl, UnitKind};
use crate::Rc;

pub(crate) mod loader;

#[derive(Debug)]
pub(crate) struct Impl {}

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

    async fn start(
        &self,
        state_manager: Sender<state::Message>,
        guard_manager: Sender<guard::Message>,
    ) {
    }

    async fn stop(
        &self,
        state_manager: Sender<state::Message>,
        guard_manager: Sender<guard::Message>,
    ) {
    }

    async fn restart(
        &self,
        state_manager: Sender<state::Message>,
        guard_manager: Sender<guard::Message>,
    ) {
        todo!()
    }
}
