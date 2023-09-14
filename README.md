# design choices

- For never changed slices, use `Rc<[T]>` (or `Rc<str>`, `Rc<Path>`);
  `Rc` is `Arc` or `Rc`, controlled by type alias.
- For io operations, use async;
- For other blocking operations, use `spawn_blocking`;
- why use `rustix` rather than `libc` and `nix`
  - rustc use this
  - safe and friendly api

# 整体架构

- 使用Actor模型，并使用channel传递信息，其范式如下：
  ```rust
  struct Message { ... }
  struct Actor { ... }
  impl Actor {
    pub fn new() -> Self
    pub fn run(self, mut rx: Receiver<Message>) -> JoinHandle<()> {
      tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
          self.serve(msg);
        }
      })
    }
    fn serve(&mut self, msg: Message) {
      // logic
    }
  }
  ```
  需要注意的是，serve内部的逻辑应当是无阻塞的，以确保actor能够及时处理消息。对于需要同步/异步阻塞的部分，需要spawn出去一个任务单独执行。

- 配置文件读取：得到impl Unit，并填充足够信息（依赖，start,stop等等）
  配置文件解析器：`File -> impl Unit`
  特性：使用基于tokio的异步io

- Unit store：存储unit静态信息
  `insert/update: (&mut UnitStore, dyn Unit) -> ()`
  `get: (&UnitStore, key: Unitkey) -> &dyn Unit`
  store
    - Actor
    - 在Unit自身的start/stop/restart任务前后插入状态检测与转换

- Jobmanager：触发job运行，触发相应unit进行状态转换，并向monitor提交监控信息
  - Job 类型：
    - Cmd
    - Blocking
    - Async

- 依赖管理：按需(?)解算依赖，并控制依次启动/停止/重启等，并按需注册状态监视
  - 需要大量访问unit store数据，可以使之与unit store在同一线程，或是作为unitstore的插件
  - 实现：
    - 无状态：简单的依赖解算
      利用队列与栈，按照依赖树顺序启动unit, 并去重
      `fn(UnitEntry, cond:FnMut(UnitEntry) -> bool, op: FnMut(UnitEntry))`

- 状态管理器：记录、调整并监视unit运行时状态信息
  - RtStatus
    - status: Running, Stopped, Stopping, Starting, Failed ...
    - monitor handle

- monitor: 事件驱动 异步
- signal handler
  - 利用tokio自带机制完成注册
  - 对于一个signal, 由于在tokio里面可以使用stream的形式处理，因此我们很容易得到以下注册方式：
    ```rust
    fn register_signal_handler<F, H>(signalkind: SignalKind, handler: H)
    where
        F: Future<Output = ()> + Send + 'static,
        H: FnOnce(Signal) -> F,
    {
        let sig = signal(signalkind).unwrap();
        tokio::spawn(handler(sig));
    }

    ```
    其中 `handler`形式如下：
    ```rust
    let handler = |mut signal| async move {
      // init here
      loop {
        signal.recv().await;
        // handle signal here
      }
    };
    ```

    运行时： signal发生 -> wait -> find pid and status -> find the event in monitor and trigger it
    注册时： action -> register events of the service/unit

    ```rust
    enum Event {
      SigChld(Pid, WaitStatus),
    }
    ```
    EventSources --mpsc-> EventHandler --mpsc-> EventConsumers(Monitor)

# units

```rust
pub trait Unit {
    fn name(&self) -> Rc<str>;
    // fn description(&self) -> Rc<str>;
    // fn documentation(&self) -> Rc<str>;

    // Kinds:
    // mount
    // service
    // timer
    // socket
    // target
    fn kind(&self) -> UnitKind;

    fn deps(&self) -> UnitDeps;

    fn start(&self， job_manager);
    fn stop(&self), job_manager;
    fn restart(&self, job_manager);
}
```

dbus
  [ ] zbus server && client

依赖解析：
    双向依赖：requires && required_by
    加载配置文件时建立依赖图

unit 启动：
    使用workqueue
        递归解算依赖并插入
          重复添加依赖问题
            [ ] 执行任务前检查unit状态
            性能问题


service:
    使用tokio异步创建进程与指令运行
    简单的状态转换（running/stopped）
      on start: stopped -> running
      on stop: running -> stopped
    [ ] 进程的监控
      on fail: running -> failed

workqueue:
    实现异步&&并发启动
      任务插入过程非阻塞
      
    默认依赖之间进行串行启动
    todo: 特殊unit：socket 自动拉起service

unit状态监控
    使用cgroup进行状态管理？


# 一些细节：

## signals

- [ ] handle signals (references: sysmaster, systemd)

## units
- mount & swap
  - [X] parse fstab
    - device
      - [ ] LABEL
      - [ ] PARTLABEL
      - [X] UUID
      - [ ] PARTUUID
      - [ ] ID
      - [X] PATH
        - [ ] valid path
    - [ ] check paths (reference: libmount)
  - [x] generate .mount unit
  - [ ] generate .swap unit
  - [ ] mount/unmount fs
  - [ ] swapon/off
  - [ ] monitor mounts and swaps

- service
  - [ ] parse .service file
  - [ ] start/stop service
  - [ ] monitor service
  
- timer
  - [ ] parse .timer file
- socket
  - [ ] parse .socket file
- target
  - [ ] parse .target file

