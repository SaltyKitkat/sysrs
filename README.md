# design choices

- For never changed slices, use `Rc<[T]>` (or `Rc<str>`, `Rc<Path>`);
- For io operations, use async;
- For other blocking operations, use `spawn_blocking`;
- why use `rustix` rather than `libc` and `nix`
  - rustc use this
  - safe and friendly api

# units

```rust
pub trait Unit {
    fn name(&self) -> Rc<str>;
    fn description(&self) -> Rc<str>;
    fn documentation(&self) -> Rc<str>;
    fn kind(&self) -> UnitKind;

    fn deps(&self) -> UnitDeps;

    fn start(&mut self);
    fn stop(&mut self);
    fn restart(&mut self);
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

