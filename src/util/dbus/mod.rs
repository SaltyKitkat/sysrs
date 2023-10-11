use std::time::Duration;

use tokio::{sync::mpsc::Sender, time::sleep};
use zbus::{dbus_interface, Connection, ConnectionBuilder};

use crate::unit::{
    state::{self, print_state, register_state_monitor, State},
    store::{self, utils::print_store, utils::start_unit, utils::stop_unit},
    UnitEntry,
};

#[derive(Debug)]
pub(crate) struct DbusServer {
    store: Sender<store::Message>,
    state: Sender<state::Message>,
}

impl DbusServer {
    pub(crate) fn new(store: Sender<store::Message>, state: Sender<state::Message>) -> Self {
        Self { store, state }
    }
}
#[dbus_interface(name = "org.sysrs.sysrs1")]
impl DbusServer {
    fn echo(&self, msg: &str) -> String {
        println!("dbus: called echo with `{msg}`");
        msg.to_owned()
    }
    async fn start_unit(&self, unit: &str) -> u8 {
        let id = UnitEntry::from(unit);
        start_unit(&self.store, id.clone()).await;
        // todo: really wait unit change to starting and then get the result
        sleep(Duration::from_millis(10)).await;
        let state = register_state_monitor(&self.state, id, |s| s == State::Starting).await;
        let state = match state.await.unwrap() {
            Ok(s) => s,
            Err(s) => dbg!(s),
        };
        state as _
    }

    async fn stop_unit(&self, unit: &str) -> u8 {
        let id = UnitEntry::from(unit);
        stop_unit(&self.store, id.clone()).await;
        // todo: really wait unit change to stop and then get the result
        sleep(Duration::from_millis(10)).await;
        let state = register_state_monitor(&self.state, id, |s| s == State::Starting).await;
        let state = match state.await.unwrap() {
            Ok(s) => s,
            Err(s) => dbg!(s),
        };
        state as _
    }

    async fn print_store(&self) {
        print_store(&self.store).await
    }
    async fn print_state(&self) {
        print_state(&self.state).await
    }

    fn get_unit(&self, unit: &str) {
        todo!()
    }
}

pub(crate) async fn connect_dbus(server: DbusServer) -> zbus::Result<Connection> {
    ConnectionBuilder::system()
        .unwrap()
        .name("org.sysrs.sysrs1")?
        .serve_at("/org/sysrs/sysrs1", server)?
        .build()
        .await
}
