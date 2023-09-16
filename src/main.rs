use std::time::Duration;

use rustix::process;
use tokio::{
    sync::mpsc::{channel, Sender},
    time::sleep,
};
use unit::state::StateManager;

use crate::{
    unit::{
        service::{Impl, Service},
        store::{start_unit, update_unit, UnitStore},
        UnitEntry, UnitImpl,
    },
    util::event::register_sig_handlers,
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

    let t0 = include_str!("unit/service/t0.service");
    let t0: Service = toml::from_str(t0).unwrap();
    dbg!(&t0);
    let t0: UnitImpl<Impl> = t0.into();
    dbg!(&t0);
    let entry = UnitEntry::from(&t0);
    let actors = Actors::new();
    println!("before insert unit");
    update_unit(&actors.store, t0).await;
    println!("after insert unit");
    println!("before start unit");
    start_unit(&actors.store, entry.clone()).await;
    println!("after start unit");
    sleep(Duration::from_secs(1)).await;
    start_unit(&actors.store, entry).await;
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
