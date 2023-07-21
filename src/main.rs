use futures::{future::ready, StreamExt};
use rustix::process;
use std::path::PathBuf;
use tokio::{fs::OpenOptions, io::BufReader};

use crate::unit::{mount::MountImpl, store::UnitStoreImpl, UnitImpl};

type Rc<T> = std::rc::Rc<T>;
// type Rc<T> = std::sync::Arc<T>;

mod fstab;
mod unit;
mod util;

fn main() {
    println!("Hello, world!");
    let uid = process::getuid();
    println!("uid: {uid:?}");
    // if uid != 0 {
    //     eprintln!("this program should run as root!");
    //     eprintln!("current uid: {}", uid);
    //     unsafe { libc::exit(1) }
    // }
    println!("running as root");
    println!("starting tokio runtime...");
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async_main());
    println!("exiting...");
}

async fn async_main() {
    println!("tokio started!");
    let mut store = UnitStoreImpl::new();
    println!("parsing fstab:");
    let path = PathBuf::from("/etc/fstab");
    let file = BufReader::new(OpenOptions::new().read(true).open(&path).await.unwrap());
    fstab::FsEntry::from_buf_reader(file)
        .map(|fs| -> Box<UnitImpl<MountImpl>> { Box::new(fs.into()) })
        .for_each(|mount| ready(store.insert(mount)))
        .await;
    dbg!(store);
    println!("tokio finished!");
}
