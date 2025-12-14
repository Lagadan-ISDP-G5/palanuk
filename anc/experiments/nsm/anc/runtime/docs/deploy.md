## Deploy runtime

### Build

```bash
cargo zigbuild --target aarch64-unknown-linux-gnu --release
```

```bash
scp -i ~/gipop_plc /Users/ander/Documents/proj/palanuk/anc/runtime/target/aarch64-unknown-linux-gnu/release/palanuk-runtime pi@172.30.40.32:/home/pi/palanuk/anc/
```
