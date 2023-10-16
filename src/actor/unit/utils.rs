use futures::{Stream, StreamExt};
use tokio::sync::mpsc::Sender;

use super::{Message, UnitObj};
use crate::{
    unit::{Unit, UnitEntry},
    Rc,
};

pub(crate) async fn update_unit(store: &Sender<Message>, unit: impl Unit + Send + Sync + 'static) {
    let entry = UnitEntry::from(&unit);
    store
        .send(Message::Update(entry, Rc::new(unit)))
        .await
        .unwrap();
}

pub(crate) async fn update_units(store: &Sender<Message>, units: impl Stream<Item = UnitObj>) {
    units
        .for_each_concurrent(None, |unit| async move {
            let entry = UnitEntry::from(unit.as_ref());
            store.send(Message::Update(entry, unit)).await.unwrap()
        })
        .await
}

pub(crate) async fn start_unit(store: &Sender<Message>, entry: UnitEntry) {
    store.send(Message::Start(entry)).await.unwrap();
}

pub(crate) async fn stop_unit(store: &Sender<Message>, entry: UnitEntry) {
    store.send(Message::Stop(entry)).await.unwrap();
}

pub(crate) async fn print_store(store: &Sender<Message>) {
    store.send(Message::DbgPrint).await.unwrap()
}
