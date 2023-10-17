use std::collections::{hash_map::Entry, HashMap};

use tokio::{
    select,
    sync::{
        mpsc::{self, Receiver, Sender},
        oneshot,
    },
    task::JoinHandle,
};

use super::{
    dep,
    state::{self, set_state},
};
use crate::{
    actor::state::set_state_with_condition,
    unit::{RtMsg, State, UnitEntry, UnitObj},
};

struct Extra {}
/// the guard during the lifetime of the unit
struct Guard {
    unit: UnitObj,
    extra: Option<Extra>,
    guard: Sender<Message>,
    state: Sender<state::Message>,
}

impl Guard {
    fn new(
        unit: UnitObj,
        extra: Option<Extra>,
        guard: Sender<Message>,
        state: Sender<state::Message>,
    ) -> Self {
        Self {
            unit,
            extra,
            guard,
            state,
        }
    }

    /// state:
    /// 1. wait deps(afters) to start(be active)
    ///     - afters: active
    ///     - requires: Starting?
    /// 2. unit start:
    ///      1. set state to starting
    ///      2. run `unit.start` (todo: prestart -> start -> post start)
    ///      3. match unit.start {
    ///             Success => set state to `Active`
    ///             Failed => set state to `Failed` and exit
    ///         }
    /// 3. wait & monitor the unit to exit \
    /// or wait stop sig and kill the unit by run `unit.stop`
    fn run(self, mut rx: Receiver<GuardMessage>) -> JoinHandle<()> {
        tokio::spawn(async move {
            let entry = UnitEntry::from(self.unit.as_ref());
            // wait deps
            if let Some(msg) = rx.recv().await {
                match msg {
                    GuardMessage::DepsReady => (),
                    GuardMessage::Stop => {
                        set_state(&self.state, entry.clone(), State::Stopped).await; // maybe unnecessary since the unit is not active here?
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
            let state = loop {
                select! {
                    msg = rx.recv() => match msg.unwrap() {
                        GuardMessage::DepsReady => todo!("log error"),
                        GuardMessage::Stop => {
                            set_state(&self.state, entry.clone(), State::Stopping).await;
                            match self.unit.stop(handle).await {
                                Ok(()) => break State::Stopped,
                                Err(()) => todo!(),
                            }
                        },
                    },
                    rt_msg = handle.wait() => match rt_msg {
                        RtMsg::Exit(state) => break state,
                        RtMsg::TriggerStart(unitentry, extra) => {
                            // todo: start the unit with extra rt info
                        }
                    },
                }
            };
            set_state(&self.state, entry.clone(), state).await;
            self.guard.send(Message::Remove(entry)).await.unwrap();
        })
    }
}

pub(crate) enum Message {
    /// Query if guard of the specific unit exists
    Contains(UnitEntry, oneshot::Sender<bool>),
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
    dep: Sender<dep::Message>,
    state: Sender<state::Message>,
}

impl GuardStore {
    pub(crate) fn new(
        self_: Sender<Message>,
        dep: Sender<dep::Message>,
        state: Sender<state::Message>,
    ) -> Self {
        Self {
            map: HashMap::new(),
            self_,
            dep,
            state,
        }
    }

    pub(crate) fn run(mut self, mut rx: Receiver<Message>) -> JoinHandle<()> {
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                match msg {
                    Message::Contains(unitentry, ret) => {
                        ret.send(self.map.contains_key(&unitentry)).unwrap();
                    }
                    Message::Insert(unitobj) => {
                        let entry = UnitEntry::from(unitobj.as_ref());
                        match self.map.entry(entry.clone()) {
                            Entry::Occupied(_) => {
                                // unit already started and running, just drop this request
                            }
                            Entry::Vacant(v) => {
                                // unit not running, create the guard to start the unit
                                let (sender, recevier) = mpsc::channel(4); // todo: remove magic number
                                self.dep
                                    .send(dep::Message::Insert(entry, unitobj.deps()))
                                    .await
                                    .unwrap();
                                Guard::new(unitobj, None, self.self_.clone(), self.state.clone())
                                    .run(recevier);
                                v.insert(sender);
                            }
                        }
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

pub(crate) async fn is_guard_exists(guard_manager: &Sender<Message>, u: UnitEntry) -> bool {
    let (s, r) = oneshot::channel();
    guard_manager.send(Message::Contains(u, s)).await.unwrap();
    r.await.unwrap()
}
