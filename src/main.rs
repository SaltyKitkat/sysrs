use std::time::Duration;

use rustix::process;
use tokio::{
    sync::mpsc::{channel, Sender},
    task::yield_now,
    time::sleep,
};

use crate::{
    unit::{
        guard::GuardManager,
        service::loader::load_service,
        store::{update_unit, UnitStore},
    },
    util::{
        dbus::{connect_dbus, DbusServer},
        event::register_sig_handlers,
    },
};
use unit::state::StateManager;

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

    let actors = Actors::new();
    let _conn = connect_dbus(DbusServer::new(actors.store.clone(), actors.state.clone()))
        .await
        .unwrap();
    println!("before insert unit");
    update_unit(
        &actors.store,
        load_service(include_str!("unit/service/t0.service")),
    )
    .await;
    update_unit(
        &actors.store,
        load_service(include_str!("unit/service/t1.service")),
    )
    .await;
    update_unit(
        &actors.store,
        load_service(include_str!("unit/service/sleep.service")),
    )
    .await;
    println!("after insert unit");
    yield_now().await;
    yield_now().await;
    sleep(Duration::from_secs(300)).await;
    println!("tokio finished!");
}

pub(crate) struct Actors {
    pub(crate) store: Sender<unit::store::Message>,
    pub(crate) state: Sender<unit::state::Message>,
    pub(crate) guard: Sender<unit::guard::Message>,
}

impl Actors {
    pub(crate) fn new() -> Self {
        // 1024 should be big enough for normal use
        const CHANNEL_LEN: usize = 1024;

        let (store, store_rx) = channel(CHANNEL_LEN);
        let (state, state_rx) = channel(CHANNEL_LEN);
        let (guard, guard_rx) = channel(CHANNEL_LEN);

        UnitStore::new(state.clone(), guard.clone()).run(store_rx);
        StateManager::new().run(state_rx);
        GuardManager::new(guard.clone(), store.clone(), state.clone()).run(guard_rx);

        Self {
            store,
            state,
            guard,
        }
    }
}
