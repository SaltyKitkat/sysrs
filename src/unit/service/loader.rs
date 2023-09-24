use super::{
    super::{UnitDeps, UnitEntry, UnitImpl},
    Impl, Kind,
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Service {
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) requires: String,
    #[serde(default)]
    pub(crate) wants: String,
    #[serde(default)]
    pub(crate) before: String,
    #[serde(default)]
    pub(crate) after: String,
    #[serde(default)]
    pub(crate) conflicts: String,
    pub(crate) kind: Kind,
    pub(crate) start: String,
    #[serde(default)]
    pub(crate) stop: String,
    #[serde(default)]
    pub(crate) restart: String,
}

impl From<Service> for UnitImpl<Impl> {
    fn from(value: Service) -> Self {
        let Service {
            name,
            requires,
            wants,
            before,
            after,
            conflicts,
            kind,
            start,
            stop,
            restart,
        } = value;
        Self::gen_test(
            name,
            UnitDeps::from_strs(requires, wants, before, after, conflicts),
            kind,
            start,
            stop,
            restart,
        )
    }
}

pub(crate) fn str_to_unitentrys(s: String) -> Box<[UnitEntry]> {
    s.split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| UnitEntry::from(s))
        .collect()
}

impl UnitDeps {
    pub(crate) fn from_strs(
        requires: String,
        wants: String,
        before: String,
        after: String,
        conflicts: String,
    ) -> Self {
        let requires = str_to_unitentrys(requires);
        let wants = str_to_unitentrys(wants);
        let before = str_to_unitentrys(before);
        let after = str_to_unitentrys(after);
        let conflicts = str_to_unitentrys(conflicts);
        Self {
            requires,
            wants,
            after,
            before,
            conflicts,
        }
    }
}
