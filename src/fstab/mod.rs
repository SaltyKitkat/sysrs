use crate::Rc;
use futures::{future::ready, Stream, StreamExt};
use std::{
    num::ParseIntError,
    path::{Path, PathBuf},
};
use tokio::io::{AsyncBufRead, AsyncBufReadExt};
use tokio_stream::wrappers::LinesStream;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FsEntry {
    pub fs_spec: Rc<Path>,
    pub mount_point: Rc<Path>,
    pub vfs_type: Rc<str>,
    pub mount_options: Rc<str>,
    pub dump: bool,
    pub fsck_order: u8,
}

impl FsEntry {
    pub fn from_buf_reader(reader: impl AsyncBufRead) -> impl Stream<Item = Self> {
        LinesStream::new(reader.lines())
            .filter_map(|line| ready(line.ok()))
            .filter(|line| ready(!line.starts_with('#')))
            .filter(|line| ready(!line.trim().is_empty()))
            .filter_map(|line| {
                ready(match line.as_str().try_into() {
                    Ok(f) => Some(f),
                    Err(e) => {
                        eprintln!(
                            "warnning: line `{line}` got parse error, ignoring... error: {e:?}"
                        );
                        None
                    }
                })
            })
    }
}

#[derive(Debug)]
pub enum Error {
    Parse(ParseIntError),
    Argnum(usize),
    // PathNotAbsolute(Rc<Path>),
}

impl From<ParseIntError> for Error {
    fn from(value: ParseIntError) -> Self {
        Self::Parse(value)
    }
}

impl From<Vec<&str>> for Error {
    fn from(value: Vec<&str>) -> Self {
        Self::Argnum(value.len())
    }
}

impl TryFrom<&[&str; 4]> for FsEntry {
    type Error = Error;

    fn try_from(value: &[&str; 4]) -> Result<Self, Self::Error> {
        let fs_spec = value[0];
        let fs_spec: PathBuf = if fs_spec.starts_with("UUID") {
            let uuid = &fs_spec["UUID".len() + 1..];
            String::from("/dev/disk/by-uuid/") + uuid
        } else {
            fs_spec.to_owned()
        }
        .into();
        let fs_spec: Rc<Path> = fs_spec.into();

        let mount_point: Rc<Path> = PathBuf::from(value[1]).into();

        // check path absolute
        // consider `tmpfs`
        // if !fs_spec.is_absolute() {
        //     return Err(Error::PathNotAbsolute(fs_spec));
        // }
        // consider `none` for swap
        // if !mount_point.is_absolute() {
        //     return Err(Error::PathNotAbsolute(mount_point));
        // }

        Ok(Self {
            fs_spec,
            mount_point,
            vfs_type: value[2].into(),
            mount_options: value[3].into(),
            dump: false,
            fsck_order: 0,
        })
    }
}

impl TryFrom<&[&str; 6]> for FsEntry {
    type Error = Error;

    fn try_from(value: &[&str; 6]) -> Result<Self, Self::Error> {
        let r: &[&str; 4] = &value[0..4].try_into().unwrap();
        let r = r.try_into()?;

        Ok(Self {
            dump: value[4].parse().map(|i: u8| i != 0)?,
            fsck_order: value[5].parse()?,
            ..r
        })
    }
}

impl TryFrom<&[&str]> for FsEntry {
    type Error = Error;

    fn try_from(value: &[&str]) -> Result<Self, Self::Error> {
        match value.len() {
            4 => {
                let v: &[&str; 4] = value.try_into().unwrap();
                v.try_into()
            }
            6 => {
                let v: &[&str; 6] = value.try_into().unwrap();
                v.try_into()
            }
            len => Err(Error::Argnum(len)),
        }
    }
}

impl TryFrom<&str> for FsEntry {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        // get the first line
        let value = value.lines().next().unwrap().trim();

        let value: Vec<&str> = value.split_ascii_whitespace().collect();
        value.as_slice().try_into()
    }
}
