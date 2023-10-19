use std::collections::{HashMap, VecDeque};

use tokio::{
    sync::mpsc::{Receiver, Sender},
    task::JoinHandle,
};

use super::{
    dep,
    guard::{self, is_guard_exists},
};
use crate::{
    actor::guard::{create_guard, guard_stop},
    unit::{UnitEntry, UnitObj},
    Rc,
};

pub(crate) mod utils;

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
    dep: Sender<dep::Message>,
    guard_manager: Sender<guard::Message>,
}

impl UnitStore {
    pub(crate) fn new(dep: Sender<dep::Message>, guard_manager: Sender<guard::Message>) -> Self {
        Self {
            map: HashMap::new(),
            dep,
            guard_manager,
        }
    }

    pub(crate) fn run(mut self, mut rx: Receiver<Message>) -> JoinHandle<()> {
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                match msg {
                    Message::DbgPrint => println!("{:#?}", self.map),
                    Message::Update(entry, unit) => {
                        println!("updating unit: {:?}", &entry);
                        if let Some(old) = self.map.insert(entry.clone(), unit.clone()) {
                            self.dep
                                .send(dep::Message::Update {
                                    id: entry,
                                    old: old.deps(),
                                    new: unit.deps(),
                                })
                                .await
                                .unwrap();
                        } else {
                            self.dep
                                .send(dep::Message::Load(entry, unit.deps()))
                                .await
                                .unwrap();
                        }
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
                                create_guard(&self.guard_manager, unit).await;
                            }
                            let mut requires = self.find_requires(unit).await;
                            while let Some(unit) = requires.pop() {
                                create_guard(&self.guard_manager, unit).await;
                            }
                            for conflict in unit.deps().conflicts.iter() {
                                guard_stop(&self.guard_manager, conflict.clone()).await;
                            }
                            create_guard(&self.guard_manager, unit.clone()).await;
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
        while let Some(required_unit) = queue.pop_front() {
            if !is_guard_exists(&self.guard_manager, required_unit.clone()).await {
                println!("finding requires...");
                if let Some(unit) = self.map.get(&required_unit) {
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
        while let Some(wanted_unit) = queue.pop_front() {
            if !is_guard_exists(&self.guard_manager, wanted_unit.clone()).await {
                println!("finding wants...");
                if let Some(unit) = self.map.get(&wanted_unit) {
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
                    todo!("handle misssing unit dep, missing: {wanted_unit}")
                }
            }
        }
        stack
    }
}
