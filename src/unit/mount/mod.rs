use rustix::fs::MountFlags;
use tokio::sync::mpsc::Sender;

use super::{UnitCommonImpl, UnitDeps, UnitImpl};
use crate::{
    fstab::{FsEntry, MountInfo},
    unit::{Unit, UnitKind},
    util::{
        job::{self, create_blocking_job},
        mount::mount,
    },
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
        let common = UnitCommonImpl {
            name,
            description: String::new().into(),
            documentation: String::new().into(),
            deps: Default::default(),
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

    fn start(&self, job_manager: &Sender<job::Message>) {
        let Self {
            common: _,
            sub: mount_info,
        } = self;
        let mount_info = mount_info.clone();
        create_blocking_job(
            job_manager.clone(),
            Box::new(move || {
                mount(mount_info, MountFlags::empty());
            }),
        )
    }

    fn stop(&self) {
        let Self {
            common: _,
            sub: mount_info,
        } = self;
        // unmount(kind.mount_point.as_ref(), UnmountFlags::empty());
    }

    fn restart(&self) {
        todo!()
    }

    fn deps(&self) -> UnitDeps {
        UnitDeps {
            requires: vec![],
            // required_by: vec![UnitEntry::from("local-fs.target")],
        }
    }
}
