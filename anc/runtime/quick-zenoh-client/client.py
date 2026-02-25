#!/usr/bin/env python3
"""Quick Zenoh pub/sub client with msgpack encoding."""

from annotationlib import ForwardRef
from lzma import FORMAT_ALONE
import zenoh
import msgpack
import threading
import sys

session = None
subscribers = {}

def on_sample(sample):
    """Callback for received samples."""
    try:
        data = msgpack.unpackb(sample.payload.to_bytes())
        print(f"\n[RECV] {sample.key_expr}: {data}")
    except Exception as e:
        print(f"\n[RECV] {sample.key_expr}: (raw) {sample.payload.to_bytes()} (decode err: {e})")
    print("> ", end="", flush=True)

def cmd_sub(key):
    """Subscribe to a key."""
    if key in subscribers:
        print(f"Already subscribed to {key}")
        return
    sub = session.declare_subscriber(key, on_sample)
    subscribers[key] = sub
    print(f"Subscribed to {key}")

def cmd_unsub(key):
    """Unsubscribe from a key."""
    if key not in subscribers:
        print(f"Not subscribed to {key}")
        return
    subscribers[key].undeclare()
    del subscribers[key]
    print(f"Unsubscribed from {key}")

def cmd_pub(key, value_str):
    """Publish to a key. Value is parsed as Python literal."""
    try:
        value = eval(value_str)
    except:
        value = value_str
    data = msgpack.packb(value)
    session.put(key, data)
    print(f"Published to {key}: {value}")

def cmd_list():
    """List active subscriptions."""
    if not subscribers:
        print("No active subscriptions")
    else:
        print("Active subscriptions:")
        for key in subscribers:
            print(f"  - {key}")

SAFE_STATE = {
    "palanuk/bstn/loopmode": 0,
    "palanuk/bstn/speed": 0.0,
    "palanuk/bstn/forcepan": 0,
    "palanuk/bstn/drivestate": 0,
    "palanuk/bstn/steercmd": 0,
}

bstn_DEFAULTS = {
    "palanuk/bstn/loopmode": 0,
    "palanuk/bstn/speed": 0.0,
    "palanuk/bstn/forcepan": 0,
    "palanuk/bstn/drivestate": 0,
    "palanuk/bstn/steercmd": 0,
}

REVERSE = {
    "palanuk/bstn/loopmode": 0,
    "palanuk/bstn/speed": 0.15,
    "palanuk/bstn/forcepan": 0,
    "palanuk/bstn/drivestate": 2,
    "palanuk/bstn/steercmd": 0,
}

FORWARD = {
    "palanuk/bstn/loopmode": 0,
    "palanuk/bstn/speed": 0.4,
    "palanuk/bstn/forcepan": 0,
    "palanuk/bstn/drivestate": 1,
    "palanuk/bstn/steercmd": 0,
}

CLOSED_LOOP_INIT_STEP1 = {
    "palanuk/bstn/loopmode": 0,
    "palanuk/bstn/speed": 0.45,
    "palanuk/bstn/forcepan": 0,
    "palanuk/bstn/drivestate": 1,
    "palanuk/bstn/steercmd": 0,
}

CLOSED_LOOP_INIT_STEP2 = {
    "palanuk/bstn/loopmode": 1,
    "palanuk/bstn/speed": 0.45,
    "palanuk/bstn/forcepan": 0,
    "palanuk/bstn/drivestate": 1,
    "palanuk/bstn/steercmd": 0,
}

CORNER_LEFT = {
    "palanuk/bstn/loopmode": 0,
    "palanuk/bstn/speed": 0.1,
    "palanuk/bstn/forcepan": 0,
    "palanuk/bstn/drivestate": 1,
    "palanuk/bstn/steercmd": 1,
}

CORNER_RIGHT = {
    "palanuk/bstn/loopmode": 0,
    "palanuk/bstn/speed": 0.1,
    "palanuk/bstn/forcepan": 0,
    "palanuk/bstn/drivestate": 1,
    "palanuk/bstn/steercmd": 2,
}


def cmd_init():
    """Initialize all bstn topics with default values."""
    print("Initializing bstn topics...")
    for key, value in bstn_DEFAULTS.items():
        data = msgpack.packb(value)
        session.put(key, data)
        print(f"  {key}: {value}")
    print("Done")

def reverse():
    for key, value in REVERSE.items():
        data = msgpack.packb(value)
        session.put(key, data)
        print(f"  {key}: {value}")
    print("Done")

def safe():
    print("Going into safe state")
    for key, value in SAFE_STATE.items():
        data = msgpack.packb(value)
        session.put(key, data)
        print(f"  {key}: {value}")
    print("Done")

def forward():
    for key, value in FORWARD.items():
        data = msgpack.packb(value)
        session.put(key, data)
        print(f"  {key}: {value}")
    print("Done")

def crr():
    for key, value in CORNER_RIGHT.items():
        data = msgpack.packb(value)
        session.put(key, data)
        print(f"  {key}: {value}")
    print("Done")

def crl():
    for key, value in CORNER_LEFT.items():
        data = msgpack.packb(value)
        session.put(key, data)
        print(f"  {key}: {value}")
    print("Done")

def pr():
    session.put("palanuk/bstn/forcepan", msgpack.packb(2))
    print("pan right")

def pl():
    session.put("palanuk/bstn/forcepan", msgpack.packb(1))
    print("pan left")

def ac():
    session.put("palanuk/bstn/drivestate", msgpack.packb(1))
    session.put("palanuk/itp/accelerate", msgpack.packb(0))
    # forward()
    session.put("palanuk/itp/accelerate", msgpack.packb(1))
    print("accelerate over bump")

def pc():
    session.put("palanuk/bstn/forcepan", msgpack.packb(0))
    print("pan center")

def cmd_closedloop_init():
    """Initialize for closed loop operation"""
    print("Initializing bstn topics...")
    for key, value in CLOSED_LOOP_INIT_STEP1.items():
        data = msgpack.packb(value)
        session.put(key, data)
        print(f"  {key}: {value}")

    for key, value in CLOSED_LOOP_INIT_STEP2.items():
        data = msgpack.packb(value)
        session.put(key, data)
        print(f"  {key}: {value}")
    print("Done")

def print_help():
    print("""
Commands:
  sub <key>           Subscribe to key (wildcards: *, **)
  unsub <key>         Unsubscribe from key
  pub <key> <value>   Publish value to key (value is eval'd as Python)
  list                List active subscriptions
  cl                  Initialize for closed loop operation
  s                   Go into safe state
  init                Initialize all bstn topics with default values
  fwd                 Creep forward
  rev                 Reverse slowly
  crr                 Corner right
  crl                 Corner left
  pl                  Pan left
  pr                  Pan right
  pc                  Pan center
  ac                  Accelerate
  help                Show this help
  quit/exit           Exit

Examples:
  sub palanuk/**
  pub palanuk/test 123
  pub palanuk/test {"speed": 0.5, "enabled": True}
  pub palanuk/test [1, 2, 3]
""")

def main():
    global session

    # router_endpoint = sys.argv[1] if len(sys.argv) > 1 else "tcp/localhost:7447"

    # print(f"Connecting to Zenoh router at {router_endpoint}...")
    config = zenoh.Config()
    # config.insert_json5("mode", '"client"')
    # config.insert_json5("connect/endpoints", f'["{router_endpoint}"]')
    session = zenoh.open(config)
    print(f"Connected! Session ID: {session.zid()}")
    print_help()

    try:
        while True:
            try:
                line = input("> ").strip()
            except EOFError:
                break

            if not line:
                continue

            parts = line.split(None, 2)
            cmd = parts[0].lower()

            if cmd in ("quit", "exit", "q"):
                break
            elif cmd == "help":
                print_help()
            elif cmd == "list":
                cmd_list()
            elif cmd == "init":
                cmd_init()
            elif cmd == "rev":
                reverse()
            elif cmd == "s":
                safe()
            elif cmd == "fwd":
                forward()
            elif cmd == "crl":
                crl()
            elif cmd == "crr":
                crr()
            elif cmd == "ac":
                ac()
            elif cmd == "pr":
                pr()
            elif cmd == "pl":
                pl()
            elif cmd == "pc":
                pc()
            elif cmd == "sub":
                if len(parts) < 2:
                    print("Usage: sub <key>")
                else:
                    cmd_sub(parts[1])
            elif cmd == "unsub":
                if len(parts) < 2:
                    print("Usage: unsub <key>")
                else:
                    cmd_unsub(parts[1])
            elif cmd == "pub":
                if len(parts) < 3:
                    print("Usage: pub <key> <value>")
                else:
                    cmd_pub(parts[1], parts[2])
            elif cmd == "cl":
                cmd_closedloop_init()
            else:
                print(f"Unknown command: {cmd}. Type 'help' for help.")

    except KeyboardInterrupt:
        print("\nInterrupted")

    finally:
        print("Closing session...")
        for sub in subscribers.values():
            sub.undeclare()
        session.close()
        print("Done")

if __name__ == "__main__":
    main()
