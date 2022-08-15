use std::{collections::HashMap, fmt::Debug};

use crate::Rc;

pub(crate) mod mount;

// #[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
// pub enum UnitKind {
//     Service,
//     Timer,
//     Mount,
//     Target,
//     Socket,
// }

// impl Display for UnitKind {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         let s = match self {
//             UnitKind::Service => "service",
//             UnitKind::Timer => "timer",
//             UnitKind::Mount => "mount",
//             UnitKind::Target => "target",
//             UnitKind::Socket => "socket",
//         };
//         f.write_str(s)
//     }
// }

pub trait UnitKind {
    fn name(&self) -> &str;
}

#[derive(Debug)]
pub struct UnitCommonImpl {
    name: Rc<str>,
    description: Rc<str>,
    documentation: Rc<str>,
    deps: UnitDeps, // todo
}

#[derive(Default, Debug)]
pub struct UnitDeps {
    // requires: (),
    // wants: (),
    // after: (),
    // before: (),
    // confilcts: (),
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct UnitEntry {
    name: Rc<str>,
}

pub trait Unit: Debug {
    fn name(&self) -> Rc<str>;
    fn description(&self) -> Rc<str>;
    fn documentation(&self) -> Rc<str>;
    fn kind(&self) -> &dyn UnitKind;

    fn start(&mut self);
    fn stop(&mut self);
    fn restart(&mut self);
}

#[derive(Debug)]
pub struct UnitImpl<KindImpl> {
    common: UnitCommonImpl,
    kind: KindImpl,
}

impl From<&dyn Unit> for UnitEntry {
    fn from(value: &dyn Unit) -> Self {
        Self {
            name: value.name().clone(),
        }
    }
}

#[derive(Debug, Default)]
pub struct UnitStore {
    map: HashMap<UnitEntry, Box<dyn Unit>>,
}

impl UnitStore {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn insert_unit(&mut self, unit: Box<dyn Unit>) {
        let entry = unit.as_ref().into();
        //. todo: handle insert existed unit
        self.map.insert(entry, unit);
    }
}
