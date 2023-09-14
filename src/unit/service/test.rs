use super::{Impl, Kind};
use crate::{
    unit::{store::update_unit, Unit, UnitCommonImpl, UnitDeps, UnitEntry, UnitImpl},
    Actors, Rc,
};

use serde::{Deserialize, Serialize};

impl UnitImpl<Impl> {
    fn gen_test(
        name: impl AsRef<str>,
        deps: UnitDeps,
        kind: Kind,
        start: impl AsRef<str>,
        stop: impl AsRef<str>,
        restart: impl AsRef<str>,
    ) -> Self {
        let null_str: Rc<str> = "".into();
        Self {
            common: UnitCommonImpl {
                name: name.as_ref().into(),
                description: null_str.clone(),
                documentation: null_str.clone(),
                deps: Rc::new(deps),
            },
            sub: Impl {
                kind: Kind::Simple,
                exec_start: start.as_ref().into(),
                exec_stop: start.as_ref().into(),
                exec_restart: start.as_ref().into(),
            },
        }
    }
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

impl UnitDeps {
    fn from_strs(
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

fn str_to_unitentrys(s: String) -> Box<[UnitEntry]> {
    s.split(',')
        .map(|s| s.trim())
        .map(|s| UnitEntry::from(s))
        .collect()
}

#[derive(Debug, Serialize, Deserialize)]
struct Service {
    name: String,
    #[serde(default)]
    requires: String,
    #[serde(default)]
    wants: String,
    #[serde(default)]
    before: String,
    #[serde(default)]
    after: String,
    #[serde(default)]
    conflicts: String,
    kind: Kind,
    start: String,
    stop: String,
    restart: String,
}

#[test]
fn test() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async move {
        let t0 = include_str!("t0.service");
        let t0: Service = toml::from_str(t0).unwrap();
        let t0: UnitImpl<Impl> = t0.into();
        let actors = Actors::new();
        update_unit(&actors.store, t0).await;
    })
}
