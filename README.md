# design choices

- For never changed slices, use `Rc<[T]>` (or `Rc<str>`, `Rc<Path>`);
- For io operations, use async;
- For other blocking operations, use `spawn_blocking`;
- why use `rustix` rather than `libc` and `nix`
  - rustc use this
  - safe and friendly api

# signals

- [ ] handle signals (references: sysmaster, systemd)

# units

- mount & swap
  - [x] parse fstab
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

# deps

- [ ] parse unit deps
- [ ] resolve deps and generate dep tree

