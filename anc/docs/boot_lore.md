## Services on the Pi

You can find systemd services at `/etc/systemd/system/`.

## Service for the pins

Service location: `/etc/systemd/system/persistent_pin_set.service`

Script location: `/usr/local/sbin/persistent_pin_set.sh`

This service configures GPIO12 as PWM0_CHAN0.

## To reload changes to the service Script

```
sudo systemctl daemon-reload
```

## dtoverlay

This should be the correct overlay for PWM, in `/boot/firmware/config.txt`

```
...
[all]
dtoverlay=pwm-2chan
...
```

`dtoverlay=pwm` doesn't seem to work with all the 4 hardware PWM channels.
