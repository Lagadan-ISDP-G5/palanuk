# Network

**Dev laptop (Wired)** - 172.30.40.127

Gateway - 255.255.0.0

**Raspberry Pi (Wired, UGREEN NIC)** - 172.30.40.32

Gateway - 255.255.255.0 (/24)

# Wireless access

Login to ssh:

```bash
ssh -i ~/gipop_plc pi@raspberrypi.local
```

If cannot resolve, try scanning:

```bash
sudo arp-scan --interface=en0 --localnet
```

and then use that IP to connect to ssh.

## Auto-reconnect

An auto-reconnect script is stored at `/home/pi/nulunabalu-reconnect.sh`.

This was configured as a cron job. The entry is in `/etc/crontab`.
