use std::{os::unix::net::UnixListener, path::Path};

use async_trait::async_trait;
use tokio::io::unix::AsyncFd;

use super::{state::State, Unit, UnitDeps, UnitHandle, UnitImpl, UnitKind};
use crate::Rc;

pub(crate) mod loader;

#[derive(Debug)]
pub(crate) struct Impl {
    path: Rc<Path>,
}

pub(super) struct Handle();
#[async_trait]
impl super::Handle for Handle {
    async fn stop(self: Box<Self>) -> Result<(), UnitHandle> {
        todo!()
    }
    async fn wait(&mut self) -> State {
        todo!()
    }
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

    async fn start(&self) -> Result<UnitHandle, ()> {
        let socket = UnixListener::bind(self.sub.path.as_ref()).unwrap();
        let fd = AsyncFd::new(socket).unwrap();
        // create_guard(
        //     &guard_manager,
        //     entry.clone(),
        //     |store, state, mut rx| async move {
        //         loop {
        //             select! {
        //                 read_ready = fd.readable() => {
        //                     read_ready.unwrap().retain_ready();
        //                     start_unit(&store, entry.clone()).await;
        //                 },
        //                 msg = rx.recv() => match msg.unwrap() {
        //                     guard::GuardMessage::Stop | guard::GuardMessage::Kill => {
        //                         drop(fd);
        //                         break State::Stopped;
        //                     }
        //                     guard::GuardMessage::RequiresReady | guard::GuardMessage::AftersReady => {
        //                         todo!()
        //                     }
        //                 }
        //             }
        //         }
        //     },
        // )
        // .await
        todo!()
    }

    async fn stop(&self, handle: UnitHandle) -> Result<(), ()> {
        handle.stop().await.or(Err(()))
    }

    async fn restart(&self, handle: UnitHandle) -> Result<UnitHandle, ()> {
        self.stop(handle).await?;
        self.start().await
    }
}
