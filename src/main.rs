use futures::{future::ready, StreamExt};
use rustix::process;
use std::{path::PathBuf, time::Duration};
use tokio::{fs::OpenOptions, io::BufReader, time::sleep};

use crate::{
    unit::{mount::Impl, store::UnitStore},
    util::event::signal::register_sig_handlers,
};

// type Rc<T> = std::rc::Rc<T>;
type Rc<T> = std::sync::Arc<T>;

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
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async_main());
    println!("exiting...");
}

async fn async_main() {
    println!("tokio started!");
    register_sig_handlers();
    let mut store = UnitStore::new();
    println!("parsing fstab:");
    let path = PathBuf::from("/etc/fstab");
    let file = BufReader::new(OpenOptions::new().read(true).open(&path).await.unwrap());
    // fstab::FsEntry::from_buf_reader(file)
    //     .map(|fs| -> Box<UnitImpl<MountImpl>> { Box::new(fs.into()) })
    //     .for_each(|mount| {
    //         store.insert(mount);
    //         ready(())
    //     })
    //     .await;
    // dbg!(store);
    // let sshd_service = UnitImpl::<ServiceImpl> {
    //     common: UnitCommonImpl {
    //         name: "sshd.service".into(),
    //         description: "".into(),
    //         documentation: "".into(),
    //         deps: todo!(),
    //     },
    //     kind: ServiceImpl {
    //         exec_start: "echo service_started".into(),
    //         exec_stop: "echo stopped".into(),
    //         exec_restart: "echo restart".into(),
    //     },
    // };
    sleep(Duration::from_secs(3)).await;
    println!("tokio finished!");
}
