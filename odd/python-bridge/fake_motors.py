"""
Fake motor telemetry publisher for trail-map testing.

Publishes msgpack-encoded left/right motor data **and encoder speeds**
to the same Zenoh topics that simple-bridge.py subscribes to.

Run alongside simple-bridge.py:
    terminal 1:  python simple-bridge.py
    terminal 2:  python fake_motors.py
"""

import math
import time

import msgpack
import zenoh

# ── Zenoh topics (must match simple-bridge.py TOPIC_MAP) ─────────────────────

LMTR_CURRENT  = "palanuk/ec/lmtr/load_current/mamps"
LMTR_POWER    = "palanuk/ec/lmtr/power/mwatts"
LMTR_BUS_V    = "palanuk/ec/lmtr/bus_voltage/mvolts"
LMTR_SHUNT_V  = "palanuk/ec/lmtr/shunt_voltage/mvolts"
LMTR_SPEED    = "palanuk/anc/lmtr-actual-speed"

RMTR_CURRENT  = "palanuk/ec/rmtr/load_current/mamps"
RMTR_POWER    = "palanuk/ec/rmtr/power/mwatts"
RMTR_BUS_V    = "palanuk/ec/rmtr/bus_voltage/mvolts"
RMTR_SHUNT_V  = "palanuk/ec/rmtr/shunt_voltage/mvolts"
RMTR_SPEED    = "palanuk/anc/rmtr-actual-speed"

DRIVESTATE    = "palanuk/bstn/drivestate"

# ── Helpers ──────────────────────────────────────────────────────────────────

BUS_VOLTAGE = 12000.0   # mV (nominal motor supply)


def pub(session: zenoh.Session, topic: str, value):
    """Publish a msgpack-encoded value (matches bridge decoder)."""
    session.put(topic, msgpack.packb(value))


def publish_motors(session, l_current: float, r_current: float,
                   l_speed: float, r_speed: float):
    """Publish a full set of motor telemetry + encoder speeds for both wheels."""
    # Power ≈ V × I  (simplified; shunt voltage is tiny)
    l_power = BUS_VOLTAGE * l_current / 1000.0   # mW
    r_power = BUS_VOLTAGE * r_current / 1000.0
    l_shunt = l_current * 0.1   # shunt resistor ≈ 0.1 Ω → mV
    r_shunt = r_current * 0.1

    pub(session, LMTR_CURRENT, l_current)
    pub(session, LMTR_POWER,   l_power)
    pub(session, LMTR_BUS_V,   BUS_VOLTAGE)
    pub(session, LMTR_SHUNT_V, l_shunt)
    pub(session, LMTR_SPEED,   l_speed)

    pub(session, RMTR_CURRENT, r_current)
    pub(session, RMTR_POWER,   r_power)
    pub(session, RMTR_BUS_V,   BUS_VOLTAGE)
    pub(session, RMTR_SHUNT_V, r_shunt)
    pub(session, RMTR_SPEED,   r_speed)


def set_drivestate(session, state: int):
    """0 = stop, 1 = forward, 2 = reverse."""
    pub(session, DRIVESTATE, state)


# ── Maneuver sequences ──────────────────────────────────────────────────────
# (name, l_speed, r_speed, l_current_mA, r_current_mA, duration_s)
#
# Speed is signed: positive = forward, negative = reverse.
# The trail map integrates these directly via differential-drive kinematics.

DEMO_SEQUENCE = [
    # Straight forward
    ("Forward",          0.5,   0.5,   500.0, 500.0, 3.0),
    # Gentle right turn  (left faster → curves right)
    ("Turn right",       0.5,   0.2,   500.0, 200.0, 2.5),
    # Straight again
    ("Forward",          0.45,  0.45,  450.0, 450.0, 2.0),
    # Gentle left turn   (right faster → curves left)
    ("Turn left",        0.2,   0.5,   200.0, 500.0, 2.5),
    # Straight
    ("Forward",          0.5,   0.5,   500.0, 500.0, 2.0),
    # Sharp right
    ("Sharp right",      0.6,   0.1,   600.0, 100.0, 1.5),
    # Straight
    ("Forward",          0.4,   0.4,   400.0, 400.0, 2.0),
    # Reverse straight
    ("Reverse",         -0.35, -0.35,  350.0, 350.0, 2.0),
    # Reverse left turn
    ("Reverse + left",  -0.15, -0.4,   150.0, 400.0, 2.0),
    # Stop
    ("Stop",             0.0,   0.0,     0.0,   0.0, 2.0),
    # Figure-eight: right arc
    ("Fig-8 right arc",  0.5,   0.25,  500.0, 250.0, 4.0),
    # Figure-eight: left arc
    ("Fig-8 left arc",   0.25,  0.5,   250.0, 500.0, 4.0),
    # Stop
    ("Stop",             0.0,   0.0,     0.0,   0.0, 1.0),
]

# ── Main loop ────────────────────────────────────────────────────────────────

def main():
    session = zenoh.open(zenoh.Config())
    print(
        "\n"
        "============================================================\n"
        "  FAKE MOTOR PUBLISHER\n"
        "============================================================\n"
        "  Publishing to: palanuk/ec/{lmtr,rmtr}/*\n"
        "                 palanuk/anc/{lmtr,rmtr}-actual-speed\n"
        "  Encoding     : msgpack\n"
        "  Looping demo maneuvers — Ctrl-C to stop\n"
        "============================================================\n"
    )

    rate_hz = 20          # publish rate
    dt = 1.0 / rate_hz

    try:
        while True:
            for name, l_spd, r_spd, l_ma, r_ma, duration in DEMO_SEQUENCE:
                ds = 0 if (l_spd == 0 and r_spd == 0) else (2 if l_spd < 0 else 1)
                set_drivestate(session, ds)
                ds_label = {0: "STOP", 1: "FWD", 2: "REV"}.get(ds, "?")
                print(
                    f"  [{ds_label}]  {name:<20s}  "
                    f"Lspd={l_spd:+.2f}  Rspd={r_spd:+.2f}  "
                    f"L={l_ma:6.1f} mA  R={r_ma:6.1f} mA  "
                    f"({duration:.1f}s)"
                )

                steps = int(duration / dt)
                for i in range(steps):
                    # Add slight noise for realism
                    noise = 0.01 * math.sin(time.time() * 7.3)
                    noise2 = 0.01 * math.sin(time.time() * 5.1)
                    c_noise_l = 10.0 * math.sin(time.time() * 7.3)
                    c_noise_r = 10.0 * math.sin(time.time() * 5.1)
                    publish_motors(
                        session,
                        max(0, l_ma + c_noise_l),
                        max(0, r_ma + c_noise_r),
                        l_spd + noise,
                        r_spd + noise2,
                    )
                    time.sleep(dt)

            print("  --- loop restart ---\n")

    except KeyboardInterrupt:
        # Clean stop
        set_drivestate(session, 0)
        publish_motors(session, 0.0, 0.0, 0.0, 0.0)
        session.close()
        print("\n  Stopped.")


if __name__ == "__main__":
    main()
