use std::{os::unix::net::UnixListener, path::Path};

use async_trait::async_trait;
use tokio::{io::unix::AsyncFd, sync::mpsc::Sender};

use super::{
    state::{self, set_state_with_condition},
    Unit, UnitDeps, UnitEntry, UnitImpl, UnitKind,
};
use crate::Rc;

#[derive(Debug)]
pub(crate) struct Impl {
    path: Rc<Path>,
}

#[async_trait]
impl Unit for UnitImpl<Impl> {
    fn name(&self) -> Rc<str> {
        self.common.name.clone()
    }

    fn description(&self) -> Rc<str> {
        self.common.description.clone()
    }

    fn documentation(&self) -> Rc<str> {
        self.common.documentation.clone()
    }

    fn kind(&self) -> UnitKind {
        UnitKind::Socket
    }

    fn deps(&self) -> Rc<UnitDeps> {
        todo!()
    }

    async fn start(&self, state_manager: Sender<state::Message>) {
        let entry = UnitEntry::from(self);
        match set_state_with_condition(&state_manager, entry.clone(), state::State::Starting, |s| {
            s.is_inactive()
        })
        .await
        {
            Ok(_) => (),
            Err(_) => return,
        };
        let socket = UnixListener::bind(self.sub.path.as_ref()).unwrap();
        let handle = tokio::spawn(async move {
            let fd = AsyncFd::new(socket).unwrap();
            fd.readable().await.unwrap().retain_ready();
            let fd = fd.into_inner();
        });
        todo!()
    }

    async fn stop(&self, state_manager: Sender<state::Message>) {
        todo!()
    }

    async fn restart(&self, state_manager: Sender<state::Message>) {
        todo!()
    }
}
