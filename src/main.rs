use std::time::Duration;

use rustix::process;
use tokio::time::sleep;

use crate::{
    actor::{
        unit::utils::{start_unit, update_units},
        Actors,
    },
    unit::UnitEntry,
    util::{
        dbus::{connect_dbus, DbusServer},
        event::register_sig_handlers,
        loader::load_units_from_dir,
    },
};

// type Rc<T> = std::rc::Rc<T>;
type Rc<T> = std::sync::Arc<T>;

mod actor;
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
    let actors = Actors::new();
    register_sig_handlers(&actors);
    // println!("parsing fstab:");
    // let path = PathBuf::from("/etc/fstab");
    // let file = BufReader::new(OpenOptions::new().read(true).open(&path).await.unwrap());
    // fstab::FsEntry::from_buf_reader(file)
    //     .map(|fs| -> Box<UnitImpl<MountImpl>> { Box::new(fs.into()) })
    //     .for_each(|mount| {
    //         store.insert(mount);
    //         ready(())
    //     })
    //     .await;
    // dbg!(store);

    println!("before insert unit");
    update_units(&actors.store, load_units_from_dir("./units").await).await;
    println!("after insert unit");
    start_unit(&actors.store, UnitEntry::from("t0.service")).await;
    // start_unit(&actors.store, UnitEntry::from("dbus-system.service")).await;
    sleep(Duration::from_secs(3)).await;
    // let _conn = connect_dbus(DbusServer::new(actors.store.clone(), actors.state.clone()))
    //     .await
    //     .unwrap();
    sleep(Duration::from_secs(300)).await;
    println!("tokio finished!");
}
