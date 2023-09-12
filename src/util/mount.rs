use rustix::fs::{mount as _mount, MountFlags};

use crate::{fstab::MountInfo, Rc};

pub(crate) fn mount(mount_info: Rc<MountInfo>, flags: MountFlags) {
    let MountInfo {
        fs_spec: source,
        mount_point: target,
        vfs_type,
        mount_options: data,
    } = mount_info.as_ref();
    _mount(
        source.as_ref(),
        target.as_ref(),
        vfs_type.as_ref(),
        flags,
        data.as_ref(),
    );
}
