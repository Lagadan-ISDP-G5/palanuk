
"""
Zenoh ↔ WebSocket bridge for UGV dashboard.

Receives msgpack-encoded sensor data from Zenoh subscribers,
maintains shared dashboard state, and forwards updates to
connected WebSocket clients in real time.  Control commands
flow the other direction: dashboard → WebSocket → Zenoh publish.
"""

import asyncio
import json
import struct
from datetime import datetime

import msgpack
import websockets
import zenoh

# ---------------------------------------------------------------------------
# Dashboard state  (single source of truth, mutated by Zenoh callbacks)
# ---------------------------------------------------------------------------

dashboard_state = {
    "navigation": {
        "obstacle": False,
        "distance": 0.0,
    },

    # ── 5V system / main board (existing) ────────────────────────────────────
    "energy": {
        "battery": 0.0,          # derived: 0–100 %
        "power_mw": 0.0,
        "power_w": 0.0,           # derived: power_mw / 1000
        "load_current_ma": 0.0,
        "current_a": 0.0,         # derived: load_current_ma / 1000
        "shunt_voltage_mv": 0.0,
        "bus_voltage_mv": 0.0,
        "voltage_v": 0.0,         # derived: bus_voltage_mv / 1000
    },

    # ── Left motor (lmtr) ── NEW ─────────────────────────────────────────────
    "lmtr": {
        "power_mw": 0.0,
        "current_ma": 0.0,
        "bus_voltage_mv": 0.0,
        "shunt_voltage_mv": 0.0,
    },

    # ── Right motor (rmtr) ── NEW ────────────────────────────────────────────
    "rmtr": {
        "power_mw": 0.0,
        "current_ma": 0.0,
        "bus_voltage_mv": 0.0,
        "shunt_voltage_mv": 0.0,
    },

    "control_state": {
        "loopmode": 0,
        "stop": 0,
        "drivestate": 0,
    },
}

DRIVE_STATE_NAMES = {0: "at_rest", 1: "forward", 2: "reverse"}

# ---------------------------------------------------------------------------
# WebSocket client tracking
# ---------------------------------------------------------------------------

connected_clients: set[websockets.WebSocketServerProtocol] = set()


# ---------------------------------------------------------------------------
# Zenoh bridge
# ---------------------------------------------------------------------------

class ZenohBridge:
    """
    Subscribes to Zenoh topics on a background thread and pushes
    updates into the asyncio event loop that drives WebSocket I/O.
    """

    # Maps Zenoh topic → (state section, state key, python type)
    TOPIC_MAP = {
        # ── Navigation ────────────────────────────────────────────────────────
        "palanuk/anc/obstacle":                    ("navigation",    "obstacle",          bool),
        "palanuk/anc/distance":                    ("navigation",    "distance",          float),

        # ── 5V system / main board ────────────────────────────────────────────
        "palanuk/ec/power/mwatts":                 ("energy",        "power_mw",          float),
        "palanuk/ec/load_current/mamps":           ("energy",        "load_current_ma",   float),
        "palanuk/ec/shunt_voltage/mvolts":         ("energy",        "shunt_voltage_mv",  float),
        "palanuk/ec/bus_voltage/mvolts":           ("energy",        "bus_voltage_mv",    float),

        # ── Left motor (lmtr) ── NEW ──────────────────────────────────────────
        "palanuk/ec/lmtr/power/mwatts":            ("lmtr",          "power_mw",          float),
        "palanuk/ec/lmtr/load_current/mamps":           ("lmtr",          "current_ma",        float),
        "palanuk/ec/lmtr/bus_voltage/mvolts":      ("lmtr",          "bus_voltage_mv",    float),
        "palanuk/ec/lmtr/shunt_voltage/mvolts":    ("lmtr",          "shunt_voltage_mv",  float),

        # ── Right motor (rmtr) ── NEW ─────────────────────────────────────────
        "palanuk/ec/rmtr/power/mwatts":            ("rmtr",          "power_mw",          float),
        "palanuk/ec/rmtr/load_current/mamps":           ("rmtr",          "current_ma",        float),
        "palanuk/ec/rmtr/bus_voltage/mvolts":      ("rmtr",          "bus_voltage_mv",    float),
        "palanuk/ec/rmtr/shunt_voltage/mvolts":    ("rmtr",          "shunt_voltage_mv",  float),

        # ── Drive control ─────────────────────────────────────────────────────
        "palanuk/bstn/drivestate":                  ("control_state", "drivestate",        int),
        "palanuk/bstn/loopmode":                    ("control_state", "loopmode",          int),
        "palanuk/bstn/stop":                        ("control_state", "stop",              int),
    }

    def __init__(self, loop: asyncio.AbstractEventLoop):
        self.loop = loop
        self.session: zenoh.Session | None = None
        self._subscribers: list = []          # prevent GC of subscriber handles

    # -- lifecycle -----------------------------------------------------------

    def open(self):
        self.session = zenoh.open(zenoh.Config())
        for topic in self.TOPIC_MAP:
            sub = self.session.declare_subscriber(topic, self._make_handler(topic))
            self._subscribers.append(sub)
        print("🔗  Zenoh session opened, subscribed to all topics")

    def close(self):
        self._subscribers.clear()
        if self.session:
            self.session.close()
            print("🔗  Zenoh session closed")

    # -- inbound (Zenoh → dashboard state → WebSocket) -----------------------

    @staticmethod
    def _decode(sample) -> object:
        """Unpack a Zenoh sample whose payload is msgpack with a 'payload' key."""
        raw = msgpack.unpackb(sample.payload.to_bytes(), raw=False)
        return raw["payload"]

    def _make_handler(self, topic: str):
        """Return a closure suitable as a Zenoh subscriber callback."""
        section, key, cast = self.TOPIC_MAP[topic]

        def handler(sample):
            try:
                value = cast(self._decode(sample))
                dashboard_state[section][key] = value

                # derived fields / logging
                self._post_update(section, key, value)

                # schedule the async broadcast on the event-loop thread
                asyncio.run_coroutine_threadsafe(
                    broadcast(section, dashboard_state[section]),
                    self.loop,
                )
            except Exception as exc:
                print(f"❌  [{topic}] {exc}")

        return handler

    @staticmethod
    def _post_update(section: str, key: str, value):
        """Handle derived state and console logging after a state mutation."""

        # ── 5V system derived fields ─────────────────────────────────────────
        if section == "energy":
            if key == "power_mw":
                dashboard_state["energy"]["power_w"] = value / 1000.0

            elif key == "bus_voltage_mv":
                dashboard_state["energy"]["voltage_v"] = value / 1000.0
                # Battery % estimated from bus voltage (5 V system → 5000 mV = 100 %)
                dashboard_state["energy"]["battery"] = min(100, max(0, (value / 5000.0) * 100))

            elif key == "load_current_ma":
                dashboard_state["energy"]["current_a"] = value / 1000.0

        # ── Left motor logging ── NEW ─────────────────────────────────────────
        elif section == "lmtr":
            print(
                f"🛞  [LMTR] power={dashboard_state['lmtr']['power_mw']:.1f} mW  "
                f"current={dashboard_state['lmtr']['current_ma']:.1f} mA  "
                f"bus={dashboard_state['lmtr']['bus_voltage_mv']:.0f} mV  "
                f"shunt={dashboard_state['lmtr']['shunt_voltage_mv']:.2f} mV"
            )

        # ── Right motor logging ── NEW ────────────────────────────────────────
        elif section == "rmtr":
            print(
                f"🛞  [RMTR] power={dashboard_state['rmtr']['power_mw']:.1f} mW  "
                f"current={dashboard_state['rmtr']['current_ma']:.1f} mA  "
                f"bus={dashboard_state['rmtr']['bus_voltage_mv']:.0f} mV  "
                f"shunt={dashboard_state['rmtr']['shunt_voltage_mv']:.2f} mV"
            )

        # ── Drive control logging ─────────────────────────────────────────────
        elif section == "control_state":
            if key == "drivestate":
                name = DRIVE_STATE_NAMES.get(value, "unknown")
                print(f"🚗  Vehicle state: {name}")
            elif key == "loopmode":
                print(f"🎛️   Control mode: {'closed' if value else 'open'} loop")
            elif key == "stop" and value == 1:
                print("🛑  Emergency stop activated")

        # ── Navigation logging ────────────────────────────────────────────────
        elif section == "navigation" and key == "obstacle" and value:
            print("⚠️   Obstacle detected!")

    # -- outbound (dashboard command → Zenoh publish) ------------------------

    def publish(self, topic_suffix: str, value, *, use_msgpack: bool = True):
        """Publish a control value onto palanuk/bstn/<topic_suffix>."""
        if not self.session:
            return False

        topic = f"palanuk/bstn/{topic_suffix}"
        try:
            if use_msgpack:
                payload = msgpack.packb(value)
            elif isinstance(value, float):
                payload = struct.pack("d", value)
            elif isinstance(value, int):
                payload = struct.pack("B", value)
            else:
                payload = str(value).encode()

            self.session.put(topic, payload)
            fmt = "msgpack" if use_msgpack else "struct"
            print(f"📤  {topic} = {value} ({fmt})")
            return True
        except Exception as exc:
            print(f"❌  publish {topic}: {exc}")
            return False


# ---------------------------------------------------------------------------
# Broadcasting helper
# ---------------------------------------------------------------------------

async def broadcast(component: str, data: dict):
    """Push a component update to every connected WebSocket client."""
    if not connected_clients:
        return

    message = json.dumps({
        "type": "component_update",
        "component": component,
        "data": data,
        "timestamp": datetime.now().isoformat(),
    })

    await asyncio.gather(
        *(client.send(message) for client in connected_clients),
        return_exceptions=True,
    )


# ---------------------------------------------------------------------------
# Dashboard commands  (WebSocket → Zenoh)
# ---------------------------------------------------------------------------

# Each command is a sequence of (topic_suffix, value) publishes.
COMMAND_TABLE = {
    "forward":  [("drivestate", 1), ("speed", 1.0), ("stop", 0)],
    "backward": [("drivestate", 2), ("speed", 1.0), ("stop", 0)],
    "left":     [("steer/left", 0.8), ("steer/right", 0.2), ("forcepan", 1)],
    "right":    [("steer/left", 0.2), ("steer/right", 0.8), ("forcepan", 2)],
    "stop":     [("stop", 1), ("drivestate", 0), ("speed", 0.0)],
}


def execute_command(bridge: ZenohBridge, command: str) -> bool:
    """Look up *command* in COMMAND_TABLE and publish each step."""
    steps = COMMAND_TABLE.get(command)
    if not steps:
        print(f"⚠️   Unknown command: {command}")
        return False

    print(f"📥  Command: {command}")
    ok = all(bridge.publish(topic, value) for topic, value in steps)
    return ok


# ---------------------------------------------------------------------------
# WebSocket server
# ---------------------------------------------------------------------------

async def ws_handler(ws: websockets.WebSocketServerProtocol, bridge: ZenohBridge):
    connected_clients.add(ws)
    print(f"✅  Dashboard connected: {ws.remote_address}")

    try:
        # send welcome + current snapshot
        await ws.send(json.dumps({
            "type": "welcome",
            "message": "Connected to UGV Zenoh Bridge",
        }))
        await ws.send(json.dumps({
            "type": "initial_data",
            "data": dashboard_state,        # now includes lmtr + rmtr sections
            "timestamp": datetime.now().isoformat(),
        }))

        async for raw in ws:
            await _handle_ws_message(ws, bridge, raw)

    except websockets.exceptions.ConnectionClosed:
        print(f"❌  Dashboard disconnected: {ws.remote_address}")
    finally:
        connected_clients.discard(ws)


async def _handle_ws_message(
    ws: websockets.WebSocketServerProtocol,
    bridge: ZenohBridge,
    raw: str,
):
    """Route a single inbound WebSocket message."""
    try:
        msg = json.loads(raw)
    except json.JSONDecodeError:
        print("❌  Invalid JSON from dashboard")
        return

    msg_type = msg.get("type")

    try:
        if msg_type == "command":
            payload = msg.get("payload")
            command = payload.get("value") if isinstance(payload, dict) else payload
            success = execute_command(bridge, command)
            await ws.send(json.dumps({
                "type": "command_response",
                "command": command,
                "success": success,
                "message": f'✅ "{command}" sent' if success else "❌ Failed",
                "timestamp": datetime.now().isoformat(),
            }))

        elif msg_type == "control_mode":
            payload = msg.get("payload")
            mode = payload.get("value") if isinstance(payload, dict) else payload
            success = bridge.publish("loopmode", mode)
            label = "Closed Loop" if mode == 1 else "Open Loop"
            await ws.send(json.dumps({
                "type": "control_mode_response",
                "success": success,
                "message": f"🎛️  Switched to {label}",
                "timestamp": datetime.now().isoformat(),
            }))

        elif msg_type == "msgpack_command":
            topic = msg.get("topic")
            value = msg.get("payload")
            success = bridge.publish(topic, value)
            await ws.send(json.dumps({
                "type": "msgpack_response",
                "success": success,
                "topic": topic,
                "timestamp": datetime.now().isoformat(),
            }))

    except Exception as exc:
        print(f"❌  Error processing {msg_type}: {exc}")


# ---------------------------------------------------------------------------
# Entrypoint
# ---------------------------------------------------------------------------

async def main():
    loop = asyncio.get_running_loop()
    bridge = ZenohBridge(loop)
    bridge.open()

    handler = lambda ws: ws_handler(ws, bridge)

    try:
        async with websockets.serve(handler, "localhost", 8081):
            print(
                "\n"
                "============================================================\n"
                "🚀  UGV ZENOH BRIDGE READY\n"
                "============================================================\n"
                "  WebSocket : ws://localhost:8081\n"
                "  Encoding  : msgpack (sensors) / struct (legacy commands)\n"
                "  Topics    : 17 subscriptions (inc. lmtr + rmtr motors)\n"
                "============================================================\n"
            )
            await asyncio.Future()       # run forever
    finally:
        bridge.close()


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("\n👋  Bridge stopped")