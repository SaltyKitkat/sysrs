use futures::{future::Either, Stream, StreamExt};
use tokio::{select, sync::mpsc::Sender};

use super::{Message, UnitObj};
use crate::{
    unit::{
        dep,
        guard::{self, create_guard, GuardMessage},
        state::{self, get_state, set_state, set_state_with_condition, State},
        Unit, UnitEntry,
    },
    Rc,
};

/// the function to start a unit
///
/// process:
/// 1. check current state is inactive
///    need: state
/// 2. set state to wait deps
///    need: state
/// 3. wait deps(afters) to start(be active)
///    need: listen to events
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
    create_guard(guard, entry.clone(), |store, state, mut rx| async move {
        if let Some(msg) = rx.recv().await {
            match msg {
                GuardMessage::DepsReady => (),
                GuardMessage::Stop => {
                    return State::Stopped;
                }
            }
        }
        // run start
        match set_state_with_condition(&state, entry.clone(), State::Starting, |s| s.is_dead())
            .await
        {
            Ok(_) => (),
            Err(_) => todo!(),
        }
        let mut handle = match unit.start().await {
            Ok(handle) => handle,
            Err(()) => return State::Failed,
        };
        set_state(&state, entry.clone(), State::Active).await;

        // started, wait stop_sig / quit
        select! {
            msg = rx.recv() => match msg.unwrap() {
                GuardMessage::DepsReady => todo!(),
                GuardMessage::Stop => {
                    set_state(&state, entry, State::Stopping).await;
                    match unit.stop(handle).await {
                        Ok(()) => State::Stopped,
                        Err(()) => todo!(),
                    }
                },
            },
            state = handle.wait() => state,
        }
    })
    .await;
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
