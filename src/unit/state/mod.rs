use std::collections::HashMap;

use super::UnitEntry;

#[derive(Clone, Copy, Debug)]
pub enum State {
    Uninit,
    Stopped,
    Failed,
    Starting,
    Running,
    Stopping,
}

pub(crate) struct StateMap {
    map: HashMap<UnitEntry, State>,
}
