## Services on the Pi

You can find systemd services at `/etc/systemd/system/`.

## Service for the pins

Service location: `/etc/systemd/system/persistent_pin_set.service`

Script location: `/usr/local/sbin/persistent_pin_set.sh`

This service configures GPIO12 as PWM0_CHAN0, and to configure GPIO13 as PWM0_CHAN1 and to start as LOW.
