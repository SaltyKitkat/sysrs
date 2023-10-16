use tokio::sync::mpsc::{channel, Sender};

use crate::actor::{dep::Dep, guard::GuardStore, state::StateStore, unit::UnitStore};

pub(crate) mod dep;
pub(crate) mod guard;
pub(crate) mod state;
pub(crate) mod unit;

pub(crate) struct Actors {
    pub(crate) store: Sender<unit::Message>,
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

        UnitStore::new(state.clone(), guard.clone()).run(store_rx);
        StateStore::new(dep.clone()).run(state_rx);
        GuardStore::new(guard.clone(), dep.clone(), state.clone()).run(guard_rx);
        Dep::new(guard.clone()).run(dep_rx);

        Self {
            store,
            state,
            guard,
            dep,
        }
    }
}
