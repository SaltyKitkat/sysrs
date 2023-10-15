use std::collections::{HashMap, VecDeque};

use tokio::{
    sync::mpsc::{Receiver, Sender},
    task::JoinHandle,
};

use super::{
    dep,
    guard::{self},
    state::{self, get_state},
    Unit, UnitEntry,
};
use crate::{unit::guard::guard_stop, Rc};

pub(crate) mod utils;

type UnitObj = Rc<dyn Unit + Send + Sync + 'static>;

pub(crate) enum Message {
    /// 用于调试 打印内部信息
    DbgPrint,
    /// 用于更新/插入对应Unit的静态信息
    Update(UnitEntry, UnitObj),
    /// 移除Store中的指定Unit
    Remove(UnitEntry),
    /// 启动指定Unit
    Start(UnitEntry),
    /// 停止指定Unit
    Stop(UnitEntry),
    /// 重启指定Unit
    Restart(UnitEntry),
}

#[derive(Debug)]
pub(crate) struct UnitStore {
    map: HashMap<UnitEntry, UnitObj>, // info in unit files
    state_manager: Sender<state::Message>,
    guard_manager: Sender<guard::Message>,
    dep: Sender<dep::Message>,
}

impl UnitStore {
    pub(crate) fn new(
        state_manager: Sender<state::Message>,
        guard_manager: Sender<guard::Message>,
        dep: Sender<dep::Message>,
    ) -> Self {
        Self {
            map: HashMap::new(),
            state_manager,
            guard_manager,
            dep,
        }
    }

    pub(crate) fn run(mut self, mut rx: Receiver<Message>) -> JoinHandle<()> {
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                match msg {
                    Message::DbgPrint => println!("{:#?}", self.map),
                    Message::Update(entry, unit) => {
                        println!("updating unit: {:?}", &entry);
                        self.map.insert(entry, unit);
                    }
                    Message::Remove(entry) => {
                        self.map.remove(&entry);
                    }
                    // start the unit and its deps
                    Message::Start(entry) => {
                        println!("starting unit: {:?}", &entry);
                        if let Some(unit) = self.map.get(&entry) {
                            // find deps
                            let mut wants = self.find_wants(unit).await;
                            while let Some(unit) = wants.pop() {
                                utils::start_unit_inner(
                                    &self.state_manager,
                                    &self.guard_manager,
                                    &self.dep,
                                    unit,
                                )
                                .await;
                            }
                            let mut requires = self.find_requires(unit).await;
                            while let Some(unit) = requires.pop() {
                                utils::start_unit_inner(
                                    &self.state_manager,
                                    &self.guard_manager,
                                    &self.dep,
                                    unit,
                                )
                                .await;
                            }
                            utils::start_unit_inner(
                                &self.state_manager,
                                &self.guard_manager,
                                &self.dep,
                                unit.clone(),
                            )
                            .await;
                        }
                    }
                    Message::Stop(entry) => {
                        println!("stopping unit: {:?}", &entry);
                        guard_stop(&self.guard_manager, entry).await;
                    }
                    Message::Restart(entry) => todo!(),
                }
            }
        })
    }

    async fn find_requires(&self, unit: &UnitObj) -> Vec<UnitObj> {
        let mut queue = VecDeque::new();
        queue.extend(unit.deps().requires.iter().cloned());
        let mut stack = Vec::new();
        while let Some(e) = queue.pop_front() {
            if get_state(&self.state_manager, e.clone()).await.is_dead() {
                println!("finding requires...");
                if let Some(unit) = self.map.get(&e) {
                    let unit = unit.clone();
                    let deps = unit.deps();
                    for dep in deps.requires.iter().cloned() {
                        println!("pushing {:?} into queue", &dep);
                        queue.push_back(dep);
                    }
                    if stack
                        .iter()
                        .all(|u_in_stack| !Rc::ptr_eq(&unit, u_in_stack))
                    {
                        stack.push(unit);
                    }
                } else {
                    todo!("handle misssing unit dep")
                }
            }
        }
        stack
    }

    async fn find_wants(&self, unit: &UnitObj) -> Vec<UnitObj> {
        let mut queue = VecDeque::new();
        queue.extend(unit.deps().wants.iter().cloned());
        let mut stack = Vec::new();
        while let Some(e) = queue.pop_front() {
            if get_state(&self.state_manager, e.clone()).await.is_dead() {
                println!("finding wants...");
                if let Some(unit) = self.map.get(&e) {
                    let unit = unit.clone();
                    let deps = unit.deps();
                    for dep in deps.wants.iter().cloned() {
                        println!("pushing {:?} into queue", &dep);
                        queue.push_back(dep);
                    }
                    if stack
                        .iter()
                        .all(|u_in_stack| !Rc::ptr_eq(&unit, u_in_stack))
                    {
                        stack.push(unit);
                    }
                } else {
                    todo!("handle misssing unit dep, missing: {e}")
                }
            }
        }
        stack
    }
}
