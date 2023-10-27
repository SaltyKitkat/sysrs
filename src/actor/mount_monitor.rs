use std::{collections::HashSet, time::Duration};

use notify::{Config, PollWatcher, Watcher};
use tap::Pipe;
use tokio::{
    fs, select,
    sync::mpsc::{self, Receiver, Sender},
    task::JoinHandle,
};

use crate::{
    unit::{Unit, UnitId},
    util::mount::{mount_point_to_unit_name, ProcMountInfoLine},
};

use super::guard;

pub(crate) enum Message {
    Registor(UnitId),
    Remove(UnitId),
}

pub(crate) struct MountMonitorStore {
    map: HashSet<UnitId>,
    guard: Sender<guard::Message>,
}

impl MountMonitorStore {
    pub(crate) fn new(guard: Sender<guard::Message>) -> Self {
        Self {
            map: Default::default(),
            guard,
        }
    }
    pub(crate) fn run(mut self, mut receiver: Receiver<Message>) -> JoinHandle<()> {
        let (tx, mut rx) = mpsc::channel(1);
        let mut watcher = PollWatcher::new(
            move |res| {
                futures::executor::block_on(async {
                    tx.send(res).await.unwrap();
                })
            },
            Config::default()
                .with_compare_contents(true)
                .with_poll_interval(Duration::from_secs_f64(0.5)),
        )
        .unwrap();
        watcher
            .watch(
                "/proc/self/mountinfo".as_ref(),
                notify::RecursiveMode::NonRecursive,
            )
            .unwrap();
        tokio::spawn(async move {
            loop {
                select! {
                    Some(msg) = receiver.recv() => {
                        match msg {
                            Message::Registor(id) => {
                                self.map.insert(id);
                            }
                            Message::Remove(id) => {
                                self.map.remove(&id);
                            }
                        }
                    }
                    Some(msg) = rx.recv() => {
                        let _ = msg.unwrap();
                        let mount_info = fs::read_to_string("/proc/self/mountinfo").await.unwrap();
                        let mount_info = mount_info
                            .lines()
                            .map(|line| ProcMountInfoLine::parse(line).mount_point.pipe(|m| UnitId::from(mount_point_to_unit_name(&m).as_str())))
                            .collect::<HashSet<_>>();
                        let dead = self.map.difference(&mount_info).cloned().collect::<Vec<_>>();
                        for unit_id in dead {
                            self.map.remove(&unit_id);
                            self.guard.send(guard::Message::NotifyDead(unit_id)).await.unwrap();
                        }
                    }
                }
            }
        })
    }
}
