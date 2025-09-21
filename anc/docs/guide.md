## Half-ass deployment

> can't be arsed to deal with CI/CD, best I can do is copy pasting scp commands atm

Deploy ANC pwm to Pi:

```
scp -i ~/gipop_plc /home/ander/Documents/lagadan/repo/torunggari/anc/experiments/pwm/target/aarch64-unknown-linux-gnu/release/anc pi@172.30.40.32:/home/pi/torunggari/anc/
```

Deploy ANC gpio to Pi:

```
scp -i ~/gipop_plc /home/ander/Documents/lagadan/repo/torunggari/anc/experiments/gpio/target/aarch64-unknown-linux-gnu/release/gpio pi@172.30.40.32:/home/pi/torunggari/anc/gpio
```

Build:

```
cross build --target aarch64-unknown-linux-gnu --release

```