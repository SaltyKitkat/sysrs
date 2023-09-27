use std::{os::unix::net::UnixListener, path::Path};

use async_trait::async_trait;
use tokio::{io::unix::AsyncFd, select, sync::mpsc::Sender};

use super::{
    guard::{self, guard_stop},
    state::{self, set_state_with_condition},
    Unit, UnitDeps, UnitEntry, UnitImpl, UnitKind,
};
use crate::{
    unit::{guard::create_guard, state::State, store::start_unit},
    Rc,
};

pub(crate) mod loader;

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

    async fn start(
        &self,
        state_manager: Sender<state::Message>,
        guard_manager: Sender<guard::Message>,
    ) {
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
        let fd = AsyncFd::new(socket).unwrap();
        create_guard(&guard_manager, entry.clone(), |store, mut rx| async move {
            loop {
                select! {
                    read_ready = fd.readable() => {
                        read_ready.unwrap().retain_ready();
                        start_unit(&store, entry.clone()).await;
                    },
                    msg = rx.recv() =>  match msg.unwrap() {
                        guard::GMessage::Stop | guard::GMessage::Kill => {
                            drop(fd);
                            break State::Stopped;
                        },
                    }
                }
            }
        })
        .await
    }

    async fn stop(
        &self,
        state_manager: Sender<state::Message>,
        guard_manager: Sender<guard::Message>,
    ) {
        let entry = UnitEntry::from(self);
        match set_state_with_condition(&state_manager, entry.clone(), state::State::Stopping, |s| {
            s.is_active()
        })
        .await
        {
            Ok(s) => (),
            Err(s) => todo!(),
        }
        guard_stop(&guard_manager, entry).await
    }

    async fn restart(
        &self,
        state_manager: Sender<state::Message>,
        guard_manager: Sender<guard::Message>,
    ) {
        todo!()
    }
}
