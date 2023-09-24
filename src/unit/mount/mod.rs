use async_trait::async_trait;
use rustix::fs::{MountFlags, UnmountFlags};
use tokio::sync::mpsc::Sender;

use super::{
    guard,
    state::{self, set_state_with_condition, State},
    UnitCommon, UnitDeps, UnitEntry, UnitImpl,
};
use crate::{
    fstab::{FsEntry, MountInfo},
    unit::{Unit, UnitKind},
    util::mount::{mount, unmount},
    Rc,
};

// #[derive(Debug, Clone)]
// pub struct Impl {
//     inner: Rc<ImplInner>,
// }
//
// #[derive(Debug)]
// struct ImplInner {
//     what: Box<Path>,
//     where_: Box<Path>,
//     type_: Box<str>,
//     options: Box<str>,
// }

pub(crate) type Impl = Rc<MountInfo>;

impl From<FsEntry> for Impl {
    fn from(value: FsEntry) -> Self {
        value.mount_info.clone()
    }
}

impl From<Impl> for UnitImpl<Impl> {
    fn from(value: Impl) -> Self {
        let name = value.mount_point.to_str().unwrap();
        let name = (if let Some(s) = name.strip_prefix('/') {
            if s.is_empty() {
                String::from('-')
            } else {
                s.replace('-', "\\x2d").replace('/', "-")
            }
        } else {
            name.replace('-', "\\x2d").replace('/', "-")
        } + ".mount")
            .into();
        let common = UnitCommon {
            name,
            description: String::new().into(),
            documentation: String::new().into(),
            deps: todo!(),
        };
        Self { common, sub: value }
    }
}

impl From<FsEntry> for UnitImpl<Impl> {
    fn from(value: FsEntry) -> Self {
        let mount_impl: Impl = value.into();
        mount_impl.into()
    }
}

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
        UnitKind::Mount
    }

    async fn start(
        &self,
        state_manager: Sender<state::Message>,
        guard_manager: Sender<guard::Message>,
    ) {
        let Self {
            common: _,
            sub: mount_info,
        } = self;
        let mount_info = mount_info.clone();
        let entry = UnitEntry::from(self);
        match set_state_with_condition(&state_manager, entry.clone(), State::Starting, |s| {
            s.is_inactive()
        })
        .await
        {
            Ok(_) => (),
            Err(_) => todo!(),
        }
        let new_state = match tokio::task::block_in_place(|| mount(mount_info, MountFlags::empty()))
        {
            Ok(_) => State::Active,
            Err(_) => State::Failed,
        };
        match set_state_with_condition(&state_manager, entry, new_state, |s| s == State::Starting)
            .await
        {
            Ok(_) => (),
            Err(_) => todo!(),
        }
    }

    async fn stop(
        &self,
        state_manager: Sender<state::Message>,
        guard_manager: Sender<guard::Message>,
    ) {
        let Self {
            common: _,
            sub: mount_info,
        } = self;
        let mount_info = mount_info.clone();
        let entry = self.into();
        match set_state_with_condition(
            &state_manager,
            UnitEntry::from(self),
            State::Stopping,
            |s| s.is_active(),
        )
        .await
        {
            Ok(_) => (),
            Err(_) => todo!(),
        }

        let new_state =
            match tokio::task::block_in_place(|| unmount(mount_info, UnmountFlags::empty())) {
                Ok(_) => State::Stopped,
                Err(_) => State::Failed,
            };
        match set_state_with_condition(&state_manager, entry, new_state, |s| s == State::Starting)
            .await
        {
            Ok(_) => (),
            Err(_) => todo!(),
        }
    }

    async fn restart(
        &self,
        state_mstate_manager: Sender<state::Message>,
        guard_manager: Sender<guard::Message>,
    ) {
        todo!()
    }

    fn deps(&self) -> Rc<UnitDeps> {
        todo!()
    }
}
