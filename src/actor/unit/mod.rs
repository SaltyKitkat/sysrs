use std::collections::HashMap;

use tokio::{
    sync::{
        mpsc::{Receiver, Sender},
        oneshot,
    },
    task::JoinHandle,
};

use super::dep;
use crate::{
    unit::{UnitId, UnitObj},
    Rc,
};

pub(crate) mod utils;

pub(crate) enum Message {
    /// 用于调试 打印内部信息
    DbgPrint,
    /// 用于更新/插入对应Unit的静态信息
    Update(UnitId, UnitObj),
    /// 移除Store中的指定Unit
    Remove(UnitId),
    Get(UnitId, oneshot::Sender<UnitObj>),
    /// 启动指定Unit
    Start(UnitId),
    /// 停止指定Unit
    Stop(UnitId),
    /// 重启指定Unit
    Restart(UnitId),
}

#[derive(Debug)]
pub(crate) struct UnitStore {
    map: HashMap<UnitId, UnitObj>, // info in unit files
    dep: Sender<dep::Message>,
}

impl UnitStore {
    pub(crate) fn new(dep: Sender<dep::Message>) -> Self {
        Self {
            map: HashMap::new(),
            dep,
            // guard_manager,
        }
    }

    pub(crate) fn run(mut self, mut rx: Receiver<Message>) -> JoinHandle<()> {
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                match msg {
                    Message::DbgPrint => println!("{:#?}", self.map),
                    Message::Update(id, unit) => {
                        println!("updating unit: {:?}", &id);
                        if let Some(old) = self.map.insert(id.clone(), unit.clone()) {
                            todo!("增量更新依赖信息");
                        } else {
                            self.dep
                                .send(dep::Message::Load(id, unit.deps()))
                                .await
                                .unwrap();
                        }
                    }
                    Message::Remove(id) => {
                        self.map.remove(&id);
                    }
                    Message::Get(id, sender) => {
                        if let Some(unitobj) = self.map.get(&id).cloned() {
                            sender.send(unitobj).ok();
                        }
                    }
                    // start the unit and its deps
                    Message::Start(id) => {
                        println!("starting unit: {:?}", &id);
                        self.dep.send(dep::Message::AddToStart(id)).await.unwrap()
                    }
                    Message::Stop(id) => {
                        println!("stopping unit: {:?}", &id);
                        self.dep.send(dep::Message::AddToStop(id)).await.unwrap()
                    }
                    Message::Restart(id) => todo!(),
                }
            }
        })
    }
}
