"""
Command Test Script
====================
Loops through every command in NAV_CMD_RECIPES, publishes it
over Zenoh, waits a configurable duration, then sends STOP
before moving to the next command.

Usage:
  python tests/command.py                  # run all commands (3 s each)
  python tests/command.py --duration 5     # 5 s per command
  python tests/command.py --list           # just print commands and exit
  python tests/command.py --only STOP DRIVE_FORWARD  # run specific commands
"""

import sys
import os
import time
import struct
import argparse

# ── make sure imports from the tests/ folder work ──
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from parking_service import (
    NavCommand,
    NAV_CMD_RECIPES,
    NAV_CMD_TOPICS,
)

# ============================================================
# Zenoh helpers (lightweight — no YOLO, no camera)
# ============================================================

def open_zenoh():
    """Open a Zenoh session and return (session, zenoh_module)."""
    try:
        import zenoh
    except ImportError:
        print("[ERROR] zenoh-python is not installed. Install with:")
        print("        pip install eclipse-zenoh")
        sys.exit(1)

    print("[INFO] Opening Zenoh session …")
    session = zenoh.open(zenoh.Config())
    print("[INFO] Zenoh session open.")
    return session, zenoh


def declare_publishers(session, topics):
    """Declare a Zenoh publisher for every topic in *topics*."""
    pubs = {}
    for t in topics:
        pubs[t] = session.declare_publisher(t)
        print(f"  publisher: {t}")
    return pubs


def publish_value(pubs, topic, value):
    """Publish a single value to a topic (int → u8, float → f64)."""
    if isinstance(value, float):
        data = struct.pack("d", value)
    else:
        data = struct.pack("B", value)
    if topic in pubs:
        pubs[topic].put(data)
    else:
        print(f"  [WARN] No publisher for {topic}")


def publish_recipe(pubs, cmd_name):
    """Publish every (topic, value) pair in the recipe for *cmd_name*."""
    recipe = NAV_CMD_RECIPES[cmd_name]
    for topic, value in recipe:
        publish_value(pubs, topic, value)


# ============================================================
# Main
# ============================================================

def main():
    parser = argparse.ArgumentParser(description="Test all ITP nav commands over Zenoh")
    parser.add_argument(
        "--duration", "-d", type=float, default=2.0,
        help="Seconds to hold each command before sending STOP (default: 3)",
    )
    parser.add_argument(
        "--list", "-l", action="store_true",
        help="Print all available commands and exit",
    )
    parser.add_argument(
        "--only", nargs="+", metavar="CMD",
        help="Run only these commands (space-separated names)",
    )
    parser.add_argument(
        "--skip-stop", action="store_true",
        help="Don't send STOP between commands (use with care!)",
    )
    args = parser.parse_args()

    # ── List mode ──
    all_commands = list(NAV_CMD_RECIPES.keys())
    if args.list:
        print(f"\nAvailable commands ({len(all_commands)}):\n")
        for name in all_commands:
            recipe = NAV_CMD_RECIPES[name]
            topics_str = ", ".join(f"{t}={v}" for t, v in recipe)
            print(f"  {name:30s} → {topics_str}")
        return

    # ── Filter commands ──
    if args.only:
        commands = []
        for c in args.only:
            c_upper = c.upper()
            if c_upper not in NAV_CMD_RECIPES:
                print(f"[ERROR] Unknown command: {c!r}")
                print(f"  Available: {', '.join(all_commands)}")
                sys.exit(1)
            commands.append(c_upper)
    else:
        # Skip internal-only commands that shouldn't be tested on hardware
        skip = {"INIT_SAFE_STATE"}
        commands = [c for c in all_commands if c not in skip]

    duration = args.duration

    # ── Open Zenoh ──
    session, zenoh_mod = open_zenoh()
    print(f"\n[INFO] Declaring publishers for {len(NAV_CMD_TOPICS)} topics …")
    pubs = declare_publishers(session, NAV_CMD_TOPICS)

    # ── Safe-state init ──
    print("\n[INFO] Publishing INIT_SAFE_STATE …")
    publish_recipe(pubs, "INIT_SAFE_STATE")
    time.sleep(1.0)

    # ── Loop through commands ──
    print(f"\n{'=' * 60}")
    print(f"  Running {len(commands)} command(s)  —  {duration:.1f}s per command")
    print(f"{'=' * 60}\n")

    try:
        for i, cmd_name in enumerate(commands, 1):
            recipe = NAV_CMD_RECIPES[cmd_name]
            topics_str = ", ".join(f"{t}={v}" for t, v in recipe)

            print(f"[{i}/{len(commands)}] {cmd_name}")
            print(f"         → {topics_str}")

            publish_recipe(pubs, cmd_name)

            # Countdown
            remaining = duration
            while remaining > 0:
                step = min(remaining, 1.0)
                time.sleep(step)
                remaining -= step
                print(f"         … {remaining:.0f}s remaining", end="\r")
            print()  # newline after countdown

            # Send STOP between commands (unless --skip-stop or this IS STOP)
            if not args.skip_stop and cmd_name != "STOP":
                print(f"         → STOP")
                publish_recipe(pubs, "STOP")
                time.sleep(2.0)

            print()

    except KeyboardInterrupt:
        print("\n\n[!] Interrupted — sending STOP …")
        publish_recipe(pubs, "STOP")
        time.sleep(0.5)

    # ── Cleanup ──
    print("[INFO] Sending final STOP + safe state …")
    publish_recipe(pubs, "STOP")
    time.sleep(0.5)

    for p in pubs.values():
        p.undeclare()
    session.close()
    print("[INFO] Zenoh session closed. Done.")


if __name__ == "__main__":
    main()
