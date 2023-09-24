use std::time::Duration;

use rustix::process;
use tokio::{
    sync::mpsc::{channel, Sender},
    task::yield_now,
    time::sleep,
};

use crate::{
    unit::{
        service::{Impl, Service},
        store::{start_unit, update_unit, UnitStore},
        UnitEntry, UnitImpl,
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

    let t0 = load_service(include_str!("unit/service/t0.service"));
    let t1 = load_service(include_str!("unit/service/t1.service"));
    let entry0 = UnitEntry::from(&t0);
    let actors = Actors::new();
    let _conn = connect_dbus(DbusServer::new(actors.store.clone(), actors.state.clone()))
        .await
        .unwrap();
    println!("before insert unit");
    update_unit(&actors.store, t0).await;
    update_unit(&actors.store, t1).await;
    println!("after insert unit");
    yield_now().await;
    yield_now().await;
    sleep(Duration::from_secs(30)).await;
    println!("tokio finished!");
}

pub(crate) struct Actors {
    pub(crate) store: Sender<unit::store::Message>,
    pub(crate) state: Sender<unit::state::Message>,
}

impl Actors {
    pub(crate) fn new() -> Self {
        const CHANNEL_LEN: usize = 4;

        let (store, store_rx) = channel(CHANNEL_LEN);
        let (state, state_rx) = channel(CHANNEL_LEN);

        UnitStore::new(state.clone()).run(store_rx);
        StateManager::new().run(state_rx);

        Self { store, state }
    }
}

fn load_service(s: &str) -> UnitImpl<Impl> {
    let t0 = s;
    let t0: Service = toml::from_str(t0).unwrap();
    dbg!(&t0);
    let t0: UnitImpl<Impl> = t0.into();
    dbg!(t0)
}
