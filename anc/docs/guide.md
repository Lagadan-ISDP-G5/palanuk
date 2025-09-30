## Half-ass deployment

> can't be arsed to deal with CI/CD, best I can do is copy pasting scp commands atm

Deploy ANC pwm to Pi:

```
scp -i ~/gipop_plc /home/ander/Documents/lagadan/repo/palanuk/anc/experiments/pwm/target/aarch64-unknown-linux-gnu/release/pwm_xprmnt pi@172.30.40.32:/home/pi/palanuk/anc/
```

Deploy ANC gpio to Pi:

```
scp -i ~/gipop_plc /home/ander/Documents/lagadan/repo/palanuk/anc/experiments/gpio/target/aarch64-unknown-linux-gnu/release/hcsr04_xprmnt pi@172.30.40.32:/home/pi/palanuk/anc/
```

Build:

```
cross build --target aarch64-unknown-linux-gnu --release

```

HC-SR04 driver example:

Build:

```
cross build --example hcsr04_xmpl --target aarch64-unknown-linux-gnu --release
```

```
scp -i ~/gipop_plc /home/ander/Documents/lagadan/repo/palanuk/anc/experiments/hcsr04/target/aarch64-unknown-linux-gnu/release/examples/hcsr04_xmpl pi@172.30.40.32:/home/pi/palanuk/anc/
```