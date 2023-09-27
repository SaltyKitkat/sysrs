use std::sync::OnceLock;

use crate::{
    unit::{UnitDeps, UnitEntry},
    Rc,
};

pub(crate) fn str_to_unitentrys(s: &str) -> Box<[UnitEntry]> {
    s.split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| UnitEntry::from(s))
        .collect()
}

static EMPTYSTR: OnceLock<Rc<str>> = OnceLock::new();
pub(crate) fn empty_str() -> Rc<str> {
    EMPTYSTR.get_or_init(|| ("".into())).clone()
}

static EMPTYDEP: OnceLock<Rc<UnitDeps>> = OnceLock::new();
pub(crate) fn empty_dep() -> Rc<UnitDeps> {
    EMPTYDEP.get_or_init(|| UnitDeps::default().into()).clone()
}
