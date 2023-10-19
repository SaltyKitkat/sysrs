use std::{mem, os::unix::net::UnixListener, path::Path};

use async_trait::async_trait;
use futures::future::pending;
use tap::Tap;
use tokio::{io::unix::AsyncFd, sync::oneshot};

use super::{ChildStdio, Extra, RtMsg, Unit, UnitDeps, UnitEntry, UnitHandle, UnitImpl, UnitKind};
use crate::Rc;

pub(crate) mod loader;

#[derive(Debug)]
pub(crate) struct Impl {
    path: Rc<Path>,
    service: UnitEntry,
}

enum RtState {
    Listening,
    Starting(oneshot::Receiver<ChildStdio>),
    Running(ChildStdio),
}

pub(super) struct Handle {
    fd: AsyncFd<UnixListener>,
    rt_state: RtState,
    service: UnitEntry,
}

#[async_trait]
impl super::Handle for Handle {
    async fn stop(self: Box<Self>) -> Result<(), UnitHandle> {
        todo!()
    }
    async fn wait(&mut self) -> RtMsg {
        // todo: monitor socket state
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
        match &mut self.rt_state {
            RtState::Listening => {
                let mut read_ready = self.fd.readable().await.unwrap();
                read_ready.retain_ready();
                let (s, r) = oneshot::channel();
                self.rt_state = RtState::Starting(r);
                return RtMsg::TriggerStart(self.service.clone(), Extra { basic_io: Some(s) });
            }
            RtState::Starting(_) => {
                if let RtState::Starting(r) = mem::replace(&mut self.rt_state, RtState::Listening) {
                    match r.await {
                        Ok(child_stdio) => {
                            self.rt_state = RtState::Running(child_stdio);
                            return RtMsg::Yield;
                        }
                        Err(_) => todo!(),
                    }
                } else {
                    unreachable!()
                }
            }
            RtState::Running((child_in, child_out, child_err)) => todo!(),
        }
        pending().await
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
        let socket = UnixListener::bind(&self.sub.path)
            .unwrap()
            .tap_mut(|s| s.set_nonblocking(true).unwrap());
        let fd = AsyncFd::new(socket).unwrap();
        Ok(Box::new(Handle {
            fd,
            rt_state: RtState::Listening,
            service: self.sub.service.clone(),
        }))
    }

    async fn stop(&self, handle: UnitHandle) -> Result<(), ()> {
        handle.stop().await.or(Err(()))
    }

    async fn restart(&self, handle: UnitHandle) -> Result<UnitHandle, ()> {
        self.stop(handle).await?;
        self.start().await
    }
}
