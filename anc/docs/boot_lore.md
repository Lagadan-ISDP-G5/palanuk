## Services on the Pi

You can find systemd services at `/etc/systemd/system/`.

## Service for the pins

Service location: `/etc/systemd/system/persistent_pin_set.service`

Script location: `/usr/local/sbin/persistent_pin_set.sh`

This service configures GPIO12 as PWM0_CHAN0.

## dtoverlay

This should be the correct overlay for PWM, in `/boot/firmware/config.txt`

```
...
[all]
dtoverlay=pwm
...
```
