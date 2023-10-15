use std::collections::HashMap;

use tokio::{
    select,
    sync::mpsc::{self, Receiver, Sender},
    task::JoinHandle,
};

use super::{
    state::{self, set_state},
    unit,
};
use crate::{
    actor::state::set_state_with_condition,
    unit::{State, UnitEntry, UnitObj},
};

struct Guard {
    unit: UnitObj,
    guard: Sender<Message>,
    state: Sender<state::Message>,
}

impl Guard {
    fn new(unit: UnitObj, guard: Sender<Message>, state: Sender<state::Message>) -> Self {
        Self { unit, guard, state }
    }
    fn run(self, mut rx: Receiver<GuardMessage>) -> JoinHandle<()> {
        tokio::spawn(async move {
            let entry = UnitEntry::from(self.unit.as_ref());
            if let Some(msg) = rx.recv().await {
                match msg {
                    GuardMessage::DepsReady => (),
                    GuardMessage::Stop => {
                        set_state(&self.state, entry.clone(), State::Stopped).await;
                        return;
                    }
                }
            }
            // run start
            match set_state_with_condition(&self.state, entry.clone(), State::Starting, |s| {
                s.is_dead()
            })
            .await
            {
                Ok(_) => (),
                Err(_) => todo!(),
            }
            let mut handle = match self.unit.start().await {
                Ok(handle) => handle,
                Err(()) => {
                    set_state(&self.state, entry.clone(), State::Active).await;
                    return;
                }
            };
            set_state(&self.state, entry.clone(), State::Active).await;

            // started, wait stop_sig / quit
            let state = select! {
                msg = rx.recv() => match msg.unwrap() {
                    GuardMessage::DepsReady => todo!(),
                    GuardMessage::Stop => {
                        set_state(&self.state, entry.clone(), State::Stopping).await;
                        match self.unit.stop(handle).await {
                            Ok(()) => State::Stopped,
                            Err(()) => todo!(),
                        }
                    },
                },
                state = handle.wait() => state,
            };
            set_state(&self.state, entry.clone(), state).await;
            self.guard.send(Message::Remove(entry)).await;
        })
    }
}

pub(crate) enum Message {
    /// Insert a guard.
    Insert(UnitObj),
    /// remove a guard \
    /// usually called by self when a gurad quits
    Remove(UnitEntry),
    /// notice all deps are ready for a specific unit \
    /// called by `Dep`
    DepsReady(UnitEntry),
    /// Send a Stop message to the specific unit guard
    Stop(UnitEntry),
}

#[derive(Debug, Clone)]
pub(crate) struct GuardStore {
    map: HashMap<UnitEntry, Sender<GuardMessage>>,
    self_: Sender<Message>,
    store: Sender<unit::Message>,
    state: Sender<state::Message>,
}

impl GuardStore {
    pub(crate) fn new(
        self_: Sender<Message>,
        store: Sender<unit::Message>,
        state: Sender<state::Message>,
    ) -> Self {
        Self {
            map: HashMap::new(),
            self_,
            store,
            state,
        }
    }

    pub(crate) fn run(mut self, mut rx: Receiver<Message>) -> JoinHandle<()> {
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                match msg {
                    Message::Insert(unitobj) => {
                        let entry = UnitEntry::from(unitobj.as_ref());
                        let (sender, recevier) = mpsc::channel(4); // todo: remove magic number
                        Guard::new(unitobj, self.self_.clone(), self.state.clone()).run(recevier);
                        self.map.insert(entry.clone(), sender); // todo: should return None, deal with Some(_) cases
                    }
                    Message::Remove(u) => {
                        self.map.remove(&u);
                    }
                    Message::DepsReady(u) => {
                        self.map
                            .get(&u)
                            .unwrap()
                            .send(GuardMessage::DepsReady)
                            .await
                            .unwrap();
                    }
                    Message::Stop(u) => {
                        self.map
                            .get(&u)
                            .unwrap()
                            .send(GuardMessage::Stop)
                            .await
                            .unwrap();
                    }
                }
            }
        })
    }
}

pub(crate) enum GuardMessage {
    DepsReady,
    Stop,
}

pub(crate) async fn create_guard(guard_manager: &Sender<Message>, u: UnitObj) {
    guard_manager.send(Message::Insert(u)).await.unwrap();
}

pub(crate) async fn guard_stop(guard_manager: &Sender<Message>, u: UnitEntry) {
    guard_manager.send(Message::Stop(u)).await.unwrap()
}
