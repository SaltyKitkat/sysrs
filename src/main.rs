use std::time::Duration;

use rustix::process;
use tokio::{
    sync::mpsc::{channel, Sender},
    time::sleep,
};
use unit::dep;

use crate::{
    unit::{
        dep::Dep,
        guard::{self, GuardStore},
        state::{self, StateStore},
        store::{self, utils::start_unit, utils::update_units, UnitStore},
        UnitEntry,
    },
    util::{
        dbus::{connect_dbus, DbusServer},
        event::register_sig_handlers,
        loader::load_units_from_dir,
    },
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

pub(crate) struct Actors {
    pub(crate) store: Sender<store::Message>,
    pub(crate) state: Sender<state::Message>,
    pub(crate) guard: Sender<guard::Message>,
    pub(crate) dep: Sender<dep::Message>,
}

impl Actors {
    pub(crate) fn new() -> Self {
        // 1024 should be big enough for normal use
        const CHANNEL_LEN: usize = 1024;

        let (store, store_rx) = channel(CHANNEL_LEN);
        let (state, state_rx) = channel(CHANNEL_LEN);
        let (guard, guard_rx) = channel(CHANNEL_LEN);
        let (dep, dep_rx) = channel(CHANNEL_LEN);

        UnitStore::new(state.clone(), guard.clone(), dep.clone()).run(store_rx);
        StateStore::new(dep.clone()).run(state_rx);
        GuardStore::new(guard.clone(), store.clone(), state.clone()).run(guard_rx);
        Dep::new(guard.clone()).run(dep_rx);

        Self {
            store,
            state,
            guard,
            dep,
        }
    }
}
