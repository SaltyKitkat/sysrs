use futures::{Stream, StreamExt};
use tokio::sync::mpsc::Sender;

use super::{Message, UnitObj};
use crate::{
    actor::{
        dep,
        guard::{self, create_guard},
        state::{self, get_state},
    },
    unit::{Unit, UnitEntry},
    Rc,
};

/// the function to start a single specific unit
///
/// process:
/// 1. check current state is inactive
/// 2. set state to wait deps
/// 3. wait deps(afters) to start(be active)
///     - afters: active
///     - requires: Starting?
/// 4. self start:
///      1. set state to starting
///      2. prestart -> start -> post start
///      3. set stare to active/failed
/// 5. notice this unit has started/failed
pub(crate) async fn start_unit_inner(
    state_manager: &Sender<state::Message>,
    guard: &Sender<guard::Message>,
    dep: &Sender<dep::Message>,
    unit: UnitObj,
) {
    let entry = UnitEntry::from(unit.as_ref());
    if !get_state(state_manager, entry.clone()).await.is_dead() {
        return;
    }
    dep.send(dep::Message::Insert(entry.clone(), unit.deps().clone()))
        .await
        .unwrap();
    create_guard(guard, unit).await;
}

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
