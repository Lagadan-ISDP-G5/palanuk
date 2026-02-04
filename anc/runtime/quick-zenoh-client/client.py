#!/usr/bin/env python3
"""Quick Zenoh pub/sub client with msgpack encoding."""

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

def print_help():
    print("""
Commands:
  sub <key>           Subscribe to key (wildcards: *, **)
  unsub <key>         Unsubscribe from key
  pub <key> <value>   Publish value to key (value is eval'd as Python)
  list                List active subscriptions
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

    print("Connecting to Zenoh...")
    config = zenoh.Config()
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
