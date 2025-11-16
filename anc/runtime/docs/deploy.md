## Deploy runtime

### Build

```bash
cross build --target aarch64-unknown-linux-gnu --release
```

```bash
scp -i ~/gipop_plc /Users/ander/Documents/proj/palanuk/anc/runtime/aarch64-unknown-linux-gnu/release/ pi@172.30.40.32:/home/pi/palanuk/anc/
```
