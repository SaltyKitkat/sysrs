use std::path::Path;

use rustix::fs::{mount, unmount, MountFlags, UnmountFlags};

use super::{UnitCommonImpl, UnitDeps, UnitEntry, UnitImpl};
use crate::{
    fstab::FsEntry,
    unit::{Unit, UnitKind},
    Rc,
};

#[derive(Debug)]
pub struct Impl {
    what: Rc<Path>,
    where_: Rc<Path>,
    type_: Rc<str>,
    options: Rc<str>,
}

impl From<FsEntry> for Impl {
    fn from(value: FsEntry) -> Self {
        Self {
            what: value.fs_spec,
            where_: value.mount_point,
            type_: value.vfs_type,
            options: value.mount_options,
        }
    }
}

impl From<Impl> for UnitImpl<Impl> {
    fn from(value: Impl) -> Self {
        let name = value.where_.to_str().unwrap();
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
        Self {
            common,
            kind: value,
        }
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

    fn start(&mut self) {
        let Self { common: _, kind } = self;
        mount(
            kind.what.as_ref(),
            kind.where_.as_ref(),
            kind.type_.as_ref(),
            MountFlags::empty(),
            kind.options.as_ref(),
        );
    }

    fn stop(&mut self) {
        let Self { common: _, kind } = self;
        unmount(kind.where_.as_ref(), UnmountFlags::empty());
    }

    fn restart(&mut self) {
        self.stop();
        self.start();
    }

    fn deps(&self) -> UnitDeps {
        UnitDeps {
            requires: vec![],
            // required_by: vec![UnitEntry::from("local-fs.target")],
        }
    }
}
