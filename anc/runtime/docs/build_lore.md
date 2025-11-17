## Build lore

Debug builds segfaults cross's llvm cc. Use zig cc.

To build:

Use `cargo-zigbuild`. https://github.com/rust-cross/cargo-zigbuild

```bash
cargo zigbuild --target aarch64-unknown-linux-gnu
```
