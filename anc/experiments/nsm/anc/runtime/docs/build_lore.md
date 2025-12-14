## Build lore

Debug builds segfaults cross's llvm cc. Use zig cc.

To build:

Use `cargo-zigbuild`. https://github.com/rust-cross/cargo-zigbuild

```bash
cargo zigbuild --target aarch64-unknown-linux-gnu
```

With each build, a `palanuk-logreader` binary is generated. You don't have to scp it to the Pi every time unless you changed its source at palanuk-runtime/src/logreader.rs
