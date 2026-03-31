"""
Vision Service — Full Pipeline (Base Station)
===============================================
Runs on the laptop. Processes YOLO detections from the RTSP camera stream
and sends navigational state/commands to AnC (on the Pi) via Zenoh.

This service does NOT:
  - Control motors or servos directly
  - Handle lane tracking (Pi does this autonomously)
  - Handle turn-right / corner detection (Pi does this autonomously)
  - Set motor speeds

This service DOES:
  - Run YOLO inference on camera frames
  - Detect bumpers → send ACCELERATE_FOR_BUMP to AnC
  - Detect valid parking slots → trigger approach/parking sequence
  - Manage approach cycle (scan, lane track, bang-bang correct)
  - Manage parking sequence (coast, align via shimmy, enter, exit)
  - Send nav commands to AnC via Zenoh

Zenoh Topics (Core — always active, per zenoh_topics.md):
  palanuk/bstn/stop       — stop / resume (u8: 1=stop, 0=resume)
  palanuk/bstn/loopmode   — loop mode (u8: 0=Open, 1=Closed)
  palanuk/bstn/speed      — speed setpoint (f64)
  palanuk/bstn/steercmd   — steering command (u8: 0=Free, 1=Hard Left, 2=Hard Right)
  palanuk/bstn/drivestate — drive state (u8: 0=Rest, 1=Forward, 2=Reverse)
  palanuk/bstn/forcepan   — camera pan (u8: 0=Center, 1=Left, 2=Right)
  palanuk/itp/accelerate  — bumper acceleration (u8: 0/1, rising edge)

  Each nav command publishes a *recipe* — a combination of the above
  topics — defined in NAV_CMD_RECIPES (parking_service.py).

Zenoh Topics (Debug — toggle via DEBUG_PUBLISH):
  anc/state             — current navigation state (JSON)
  anc/detections        — per-frame detection summary (JSON)

Overall States:
  LANE_FOLLOWING        — Pi handles lane tracking; vision watches for bumpers
  APPROACH_PARKING      — Scanning, parking, alignment, exit — all phases
  FINISHED              — Done
  ERROR                 — Something went wrong
"""

import cv2
import time
import json
import os
import logging
import threading
from enum import Enum, auto
from datetime import datetime
from typing import Optional, List, Dict, Any
from ultralytics import YOLO
import numpy as np
import torch


def _auto_device() -> str:
    """Pick the best available inference device."""
    if torch.cuda.is_available():
        return "0"
    if torch.backends.mps.is_available():
        return "mps"
    return "cpu"

from parking_service import (
    ParkingConfig,
    NavCommand,
    NAV_CMD_TOPICS,
    parse_detections,
    Detection,
    DebounceTracker,
    classify_all_slots,
)

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(name)s] %(levelname)s: %(message)s",
)
logger = logging.getLogger("VisionService")


# ============================================================
# Overall States
# ============================================================

class OverallState(Enum):
    INIT = auto()
    LANE_FOLLOWING = auto()
    APPROACH_PARKING = auto()
    FINISHED = auto()
    ERROR = auto()


# ============================================================
# Configuration
# ============================================================

class VisionConfig:
    """Vision service configuration."""

    # Model
    MODEL_PATH: str = "best.pt"
    IMG_SIZE: int = 640
    CONF_THRES: float = 0.6
    DEVICE: str = _auto_device()
    TASK: str = "segment"

    # FP16 / TensorRT (only effective on CUDA)
    USE_FP16: bool = True
    USE_TENSORRT: bool = False
    TENSORRT_ENGINE_PATH: str = "tests/best.engine"

    # Stream
    STREAM_URL: str = "rtsp://raspberrypi.local:8554/camera"
    MAX_RETRIES: int = 5

    # Processing
    USE_ONLY_BOXES: bool = True

    # Class names (must match YOLO model)
    CLASS_BUMPER: str = "bumper"
    CLASS_PARKING_SLOT: str = "parking_slot"
    CLASS_P_SIGN: str = "parking_signboard"
    CLASS_DISABLED_SIGN: str = "disabled person signboard"
    CLASS_CONE: str = "cone"

    # Bumper detection — send ACCELERATE_FOR_BUMP when bumper y2 is
    # in the bottom portion of the frame (close to robot).
    # Value is fraction of frame height from the top; 0.95 = bottom 5%.
    BUMPER_Y2_THRESHOLD: float = 0.95  # bumper base in bottom 5% of frame

    # Parking slot detection — minimum area as fraction of frame
    VALID_PARKING_AREA_THRESHOLD: float = 0.12  # 10% of frame

    # Debounce frame counts
    BUMPER_DEBOUNCE_FRAMES: int = 3
    SLOT_DEBOUNCE_FRAMES: int = 3

    # Approach parking — phase durations (seconds)
    APPROACH_PAN_CENTER_SETTLE_S: float = 2.0  # wait for camera to reach center before moving
    APPROACH_PAN_CENTER_S: float = 0.5         # drive forward with lane tracking
    APPROACH_STOP_S: float = 0.5               # stopped, waiting before panning right
    APPROACH_PAN_RIGHT_S: float = 2.0          # stopped, scanning for slot

    # Bang-bang correction
    BANGBANG_STARTUP_S: float = 1.0            # buffer before checking motors — lets AnC start the correction
    BANGBANG_TIMEOUT_S: float = 3.0            # safety timeout if motors never reach 0
    MOTOR_SPEED_ZERO_THRESHOLD: float = 0.01   # consider motor "stopped" below this

    # Parking trigger thresholds (in "right" phase)
    PARK_READY_AREA: float = 0.12              # slot area fraction to trigger parking
    PARK_READY_Y2: float = 0.85                # slot y2 / frame_height to trigger parking

    # Parking phase durations (seconds)
    PARK_PAN_SETTLE_S: float = 1.5             # wait for camera pan to settle
    PARK_COAST_S: float = 0.8                  # lane track forward to get alongside slot
    PARK_STOP_S: float = 0.5                   # settle after stopping
    PARK_TURN_S: float = 2.0                   # wait for 90° turn to complete
    PARK_ENTER_S: float = 0.8                  # drive forward into slot (with P-sign guidance)
    PARK_ENTER_DEADBAND_PX: float = 20.0       # P sign centre tolerance for driving straight
    PARK_ENTER_CLOSE_Y: float = 0.85           # P sign y2/frame_h to consider "in slot"
    PARK_ENTER_MIN_S: float = 0.5              # minimum drive time before checking if in slot
    PARK_DONE_S: float = 4.0                   # settle after parking
    PARK_EXIT_FWD_S: float = 1.5               # drive forward out of slot

    # Alignment
    PARK_CHECK_TIMEOUT_S: float = 2.0           # wait this long before reversing to reframe
    PARK_CHECK_REVERSE_S: float = 0.2          # reverse duration to bring slot back into view
    PARK_ALIGN_TOLERANCE_PX: int = 30          # P_sign.cx vs slot.cx tolerance
    ADJUST_PX_TO_S: float = 0.005              # pixel offset to drive duration factor

    # Zenoh topics — core (always active)
    # Nav command topics are defined per-command in NAV_CMD_RECIPES (parking_service.py)

    # Zenoh topics — debug only (toggle with DEBUG_PUBLISH)
    DEBUG_PUBLISH: bool = False                      # set True to publish state & detections
    ZENOH_TOPIC_STATE: str = "anc/state"             # debug: current overall state
    ZENOH_TOPIC_DETECTIONS: str = "anc/detections"   # debug: per-frame detection summary

    # Logging
    LOG_DIR: str = "tests/logs"
    LOG_DETECTIONS: bool = True


# ============================================================
# Zenoh Wrapper
# ============================================================

class ZenohBridge:
    """
    Wrapper around Zenoh pub/sub.
    Falls back to console logging if zenoh is not available.
    """

    def __init__(self):
        self.session = None
        self.publishers: Dict[str, Any] = {}
        self.subscribers: Dict[str, Any] = {}
        self._ack_buffer: List[dict] = []
        self._zenoh_available = False

        try:
            import zenoh
            self._zenoh_module = zenoh
            self._zenoh_available = True
            logger.info("Zenoh module available")
        except ImportError:
            logger.warning("Zenoh not installed — console fallback mode")

    def open(self):
        if not self._zenoh_available:
            logger.info("ZenohBridge: console-only mode")
            return
        try:
            config = self._zenoh_module.Config()
            self.session = self._zenoh_module.open(config)
            logger.info("Zenoh session opened")
        except Exception as e:
            logger.error(f"Zenoh open failed: {e}")
            self._zenoh_available = False

    def close(self):
        if self.session is not None:
            self.session.close()
            logger.info("Zenoh session closed")

    def declare_publisher(self, topic: str):
        if self.session is not None:
            self.publishers[topic] = self.session.declare_publisher(topic)
            logger.info(f"Publisher declared: {topic}")

    def declare_subscriber(self, topic: str, callback):
        if self.session is not None:
            self.subscribers[topic] = self.session.declare_subscriber(topic, callback)
            logger.info(f"Subscriber declared: {topic}")

    def publish(self, topic: str, data: Any):
        """Publish dict or string to a Zenoh topic (debug / telemetry)."""
        payload = json.dumps(data) if isinstance(data, dict) else str(data)

        if topic in self.publishers:
            self.publishers[topic].put(payload)
        else:
            logger.info(f"[PUB {topic}] {payload}")

    def publish_bytes(self, topic: str, data: bytes):
        """Publish raw bytes to a Zenoh topic (nav commands)."""
        if topic in self.publishers:
            self.publishers[topic].put(data)
        else:
            logger.info(f"[PUB {topic}] {data.hex()}")

    def get_acks(self) -> List[dict]:
        """Return and clear buffered ACK messages from AnC."""
        msgs = list(self._ack_buffer)
        self._ack_buffer.clear()
        return msgs

    def _on_ack(self, sample):
        try:
            msg = json.loads(sample.payload.decode())
            self._ack_buffer.append(msg)
            logger.debug(f"ACK received: {msg}")
        except Exception as e:
            logger.warning(f"Bad ACK payload: {e}")


# ============================================================
# Threaded Frame Grabber
# ============================================================

class FrameGrabber:
    """Continuously grabs frames in a background thread.
    The main loop always gets the most recent frame, eliminating
    RTSP buffer lag."""

    def __init__(self, url: str):
        # Force TCP transport to reduce packet loss on Wi-Fi
        os.environ["OPENCV_FFMPEG_CAPTURE_OPTIONS"] = "rtsp_transport;tcp"
        self.url = url
        self.cap = cv2.VideoCapture(url, cv2.CAP_FFMPEG)
        self.cap.set(cv2.CAP_PROP_BUFFERSIZE, 1)
        self._ret = False
        self._frame = None
        self._lock = threading.Lock()
        self._running = True
        self._thread = threading.Thread(target=self._run, daemon=True)
        self._thread.start()

    def _run(self):
        while self._running:
            ret, frame = self.cap.read()
            if not ret:
                continue
            with self._lock:
                self._ret, self._frame = ret, frame

    def read(self):
        with self._lock:
            if self._frame is None:
                return False, None
            return self._ret, self._frame.copy()

    def isOpened(self) -> bool:
        return self.cap.isOpened()

    def release(self):
        self._running = False
        self._thread.join(timeout=2)
        self.cap.release()

    def reconnect(self):
        """Release and re-open the stream."""
        self.release()
        self.cap = cv2.VideoCapture(self.url, cv2.CAP_FFMPEG)
        self.cap.set(cv2.CAP_PROP_BUFFERSIZE, 1)
        self._running = True
        self._thread = threading.Thread(target=self._run, daemon=True)
        self._thread.start()


# ============================================================
# Motor Speed Monitor
# ============================================================

class MotorSpeedMonitor:
    """
    Subscribes to actual motor speeds from AnC (encoder feedback).
    Used to detect when a maneuver has physically completed.

    Zenoh topics:
      palanuk/anc/lmtr-actual-speed  (f32, normalized RPM)
      palanuk/anc/rmtr-actual-speed  (f32, normalized RPM)
    """

    TOPIC_LMTR = "palanuk/anc/lmtr-actual-speed"
    TOPIC_RMTR = "palanuk/anc/rmtr-actual-speed"

    def __init__(self, threshold: float = 0.01):
        self.lmtr_speed: float = 0.0
        self.rmtr_speed: float = 0.0
        self.threshold = threshold
        self._lock = threading.Lock()

    @staticmethod
    def _extract_speed(unpacked):
        """Extract a numeric speed from a msgpack-decoded value (may be a dict or scalar)."""
        if isinstance(unpacked, dict):
            return abs(float(next(iter(unpacked.values()))))
        return abs(float(unpacked))

    def on_lmtr_speed(self, sample):
        try:
            import msgpack
            raw = bytes(sample.payload) if not isinstance(sample.payload, bytes) else sample.payload
            speed = msgpack.unpackb(raw)
            with self._lock:
                self.lmtr_speed = self._extract_speed(speed)
        except Exception as e:
            logger.warning(f"Bad lmtr speed payload: {e}")

    def on_rmtr_speed(self, sample):
        try:
            import msgpack
            raw = bytes(sample.payload) if not isinstance(sample.payload, bytes) else sample.payload
            speed = msgpack.unpackb(raw)
            with self._lock:
                self.rmtr_speed = self._extract_speed(speed)
        except Exception as e:
            logger.warning(f"Bad rmtr speed payload: {e}")

    def both_stopped(self) -> bool:
        with self._lock:
            return (self.lmtr_speed < self.threshold and
                    self.rmtr_speed < self.threshold)


# ============================================================
# Color Generator
# ============================================================

def generate_colors(num_classes: int) -> Dict[int, tuple]:
    np.random.seed(42)
    return {i: tuple(map(int, np.random.randint(0, 255, 3))) for i in range(num_classes)}


# ============================================================
# MAIN VISION SERVICE
# ============================================================

class VisionService:
    """
    Main vision service. Runs YOLO, manages navigation state machine,
    publishes commands and detections to AnC via Zenoh.
    """

    def __init__(self, vcfg: VisionConfig, pcfg: ParkingConfig):
        self.vcfg = vcfg
        self.pcfg = pcfg

        # State
        self.state = OverallState.INIT

        # Model
        self.model: Optional[YOLO] = None
        self.class_names: dict = {}
        self.class_colors: dict = {}

        # Camera
        self.grabber: Optional[FrameGrabber] = None

        # Zenoh
        self.zenoh = ZenohBridge()

        # Parking alignment state (for shimmy adjustment)
        self._park_offset_px: float = 0.0       # P_sign.cx - slot.cx from last check
        self._park_offset_dir: int = 0          # +1 = need forward, -1 = need reverse
        self._adjust_drive_duration: float = 0.0  # computed from offset

        # Debounce trackers
        self.bumper_tracker = DebounceTracker(
            found_thresh=vcfg.BUMPER_DEBOUNCE_FRAMES, lost_thresh=3
        )
        self.any_slot_tracker = DebounceTracker(
            found_thresh=vcfg.SLOT_DEBOUNCE_FRAMES, lost_thresh=5
        )
        self.valid_slot_tracker = DebounceTracker(
            found_thresh=vcfg.SLOT_DEBOUNCE_FRAMES, lost_thresh=5
        )

        # Motor speed monitor (for bang-bang completion detection)
        self.motor_monitor = MotorSpeedMonitor(
            threshold=vcfg.MOTOR_SPEED_ZERO_THRESHOLD
        )

        # Approach parking — phase tracking
        self._approach_phase: str = "right"
        self._approach_phase_time: float = 0.0
        self._bangbang_motor_seen: bool = False  # two-phase: True once motors move
        self._park_check_reversed: bool = False  # True after one reverse attempt

        # Logging
        self.log_file = None
        self.run_id = datetime.now().strftime("%Y-%m-%d_%H-%M-%S")

        # Stats
        self.frame_id = 0
        self.processed_count = 0
        self._prev_time: float = time.time()
        self._fps: float = 0.0
        self._infer_ms: float = 0.0

    # ----------------------------------------------------------
    # Lifecycle
    # ----------------------------------------------------------

    def start(self):
        """Initialize all subsystems and enter main loop."""
        logger.info("=" * 60)
        logger.info("VISION SERVICE STARTING (Base Station)")
        logger.info("=" * 60)

        self._load_model()
        self._connect_camera()
        self._setup_zenoh()
        self._setup_logging()

        self._set_state(OverallState.LANE_FOLLOWING)

        try:
            self._main_loop()
        except KeyboardInterrupt:
            logger.info("Interrupted by user")
        finally:
            self._cleanup()

    def _main_loop(self):
        """Core frame processing loop."""
        while self.state not in (OverallState.FINISHED, OverallState.ERROR):
            ret, frame = self.grabber.read()
            if not ret or frame is None:
                time.sleep(0.01)  # brief wait for grabber to populate
                continue

            self.frame_id += 1

            frame_resized = cv2.resize(frame, (self.vcfg.IMG_SIZE, self.vcfg.IMG_SIZE))

            # Inference
            t0 = time.time()
            result = self.model.predict(
                source=frame_resized,
                imgsz=self.vcfg.IMG_SIZE,
                conf=self.vcfg.CONF_THRES,
                device=self.vcfg.DEVICE,
                half=self.vcfg.USE_FP16,
                verbose=False,
            )[0]
            self._infer_ms = (time.time() - t0) * 1000

            detections = parse_detections(result, self.class_names, self.vcfg.CONF_THRES)

            # Log
            self._log_frame(detections)

            # Publish raw detections (debug only)
            if self.vcfg.DEBUG_PUBLISH:
                self._publish_detections(detections)

            # Process state machine
            self._process_state(result, detections)


            # FPS
            now = time.time()
            self._fps = 1.0 / max(now - self._prev_time, 1e-6)
            self._prev_time = now

            # Visualize locally
            self._visualize(frame_resized, result, detections)

            self.processed_count += 1

            if cv2.waitKey(1) & 0xFF == 27:
                logger.info("ESC pressed — exiting")
                break

    # ----------------------------------------------------------
    # State Processing
    # ----------------------------------------------------------

    def _process_state(self, result, detections: List[Detection]):
        """Route to the appropriate state handler."""
        if self.state == OverallState.LANE_FOLLOWING:
            self._state_lane_following(detections)
        elif self.state == OverallState.APPROACH_PARKING:
            self._state_approach_parking(result)

    def _state_lane_following(self, detections: List[Detection]):
        """
        LANE_FOLLOWING: Pi is lane tracking autonomously.
        Vision watches for bumpers and parking slots.

        Bumper: sends ACCELERATE_FOR_BUMP once when bumper y2 is near
                the bottom of the frame (close to robot).
        Parking: when any parking slot is detected, stops and pans right
                 to enter APPROACH_PARKING.
        """
        frame_area = self.vcfg.IMG_SIZE ** 2
        frame_h = self.vcfg.IMG_SIZE

        # ── Bumper detection ──
        # Trigger when bumper base (y2) is in the bottom 5% of the frame
        # (very close to the robot) so AnC accelerates to cross it.
        bumpers = [
            d for d in detections
            if d.class_name == self.vcfg.CLASS_BUMPER
            and d.y2 >= frame_h * self.vcfg.BUMPER_Y2_THRESHOLD
        ]

        bmp_status = self.bumper_tracker.update(len(bumpers) > 0)
        if bmp_status == "CONFIRMED_FOUND":  # fires only once
            b = bumpers[0]
            logger.info(f"Bumper detected (y2={b.y2:.0f}, threshold={frame_h * self.vcfg.BUMPER_Y2_THRESHOLD:.0f})")
            self._send_nav(NavCommand(
                command="ACCELERATE_FOR_BUMP",
                metadata={
                    "center_x": round(b.center_x, 1),
                    "y2": round(b.y2, 1),
                    "area_pct": round(b.area / frame_area, 4),
                },
            ))
            # Reset accelerate after 1.5 seconds
            def _reset_accel():
                logger.info("Resetting ACCELERATE_FOR_BUMP after 1.5s")
                self.zenoh.publish_bytes("palanuk/itp/accelerate", NavCommand._encode(0))
            threading.Timer(1.5, _reset_accel).start()

        # ── Parking slot detection ──
        # When ANY parking slot is seen (valid or invalid, any size),
        # pan camera right and transition to APPROACH_PARKING.
        # No area threshold — the slot could still be far away.
        any_slots = [
            d for d in detections
            if d.class_name == self.pcfg.CLASS_PARKING_SLOT
        ]

        slot_status = self.any_slot_tracker.update(len(any_slots) > 0)
        if slot_status == "CONFIRMED_FOUND":  # fires only once
            best = max(any_slots, key=lambda s: s.area)
            logger.info(
                f"Parking slot(s) detected "
                f"(area={best.area/frame_area:.2%}, "
                f"cx={best.center_x:.1f}, cy={best.center_y:.1f})"
            )
            logger.info("Entering APPROACH_PARKING — pan right and scan immediately")
            self._send_nav(NavCommand(command="STOP"))
            self._send_nav(NavCommand(command="PAN_CAMERA_RIGHT"))
            self._approach_phase = "right"
            self._approach_phase_time = time.time()
            self._set_state(OverallState.APPROACH_PARKING)

    def _state_approach_parking(self, result):
        """
        APPROACH_PARKING: Full parking sequence.

        Approach cycle (scan for valid slot):
          right → settling → center → bangbang → stopping → right → ...

        Once valid slot is close enough (area + y2 thresholds):
          park_pan_centre → park_coast → park_coast_stop → park_coast_bb
          → park_face_slot → park_check
          → (aligned) park_enter → park_done → park_rotate_1 → park_rotate_2
             → park_exit_fwd → park_exit_turn → park_exit_bb → LANE_FOLLOWING
          → (not aligned) adjust_back → adjust_bb1 → adjust_drive
             → adjust_drive_stop → adjust_bb2 → adjust_face_slot → park_check
        """
        now = time.time()
        elapsed = now - self._approach_phase_time

        # ══════════════════════════════════════════════════════════
        # APPROACH CYCLE — scan for valid slot
        # ══════════════════════════════════════════════════════════

        if self._approach_phase == "right":
            # Stopped and panned right — scan for valid slot
            self._send_nav(NavCommand(command="STOP"))

            detections = parse_detections(result, self.class_names, self.vcfg.CONF_THRES)
            frame_area = self.vcfg.IMG_SIZE ** 2
            frame_h = self.vcfg.IMG_SIZE

            slot_status_map = classify_all_slots(detections, self.pcfg)
            frame_mid_x = self.vcfg.IMG_SIZE / 2

            # ── Slot validation logging ──
            all_slots = [d for d in detections if d.class_name == self.pcfg.CLASS_PARKING_SLOT]
            if all_slots:
                for i, s in enumerate(all_slots):
                    area_pct = s.area / frame_area
                    y2_pct = s.y2 / frame_h
                    status_str = slot_status_map.get(id(s), "UNKNOWN")
                    side = "RIGHT" if s.center_x >= frame_mid_x else "LEFT"
                    area_ok = area_pct >= self.vcfg.VALID_PARKING_AREA_THRESHOLD
                    reasons = []
                    if status_str != "VALID":
                        reasons.append(f"status={status_str}")
                    if not area_ok:
                        reasons.append(
                            f"area={area_pct:.1%}<{self.vcfg.VALID_PARKING_AREA_THRESHOLD:.0%}")
                    if side == "LEFT":
                        reasons.append("left-of-centre")
                    reject = f" REJECTED({', '.join(reasons)})" if reasons else " OK"
                    logger.info(
                        f"Slot[{i}]: status={status_str} side={side} "
                        f"area={area_pct:.1%} y2={y2_pct:.1%} "
                        f"bbox=({s.x1:.0f},{s.y1:.0f},{s.x2:.0f},{s.y2:.0f}){reject}"
                    )
            else:
                logger.debug("Right scan: no slots detected")

            valid_slots = [
                d for d in detections
                if d.class_name == self.pcfg.CLASS_PARKING_SLOT
                and slot_status_map.get(id(d), "UNKNOWN") == "VALID"
                and d.area / frame_area >= self.vcfg.VALID_PARKING_AREA_THRESHOLD
                and d.center_x >= frame_mid_x  # slot is on the right side
            ]

            status = self.valid_slot_tracker.update(len(valid_slots) > 0)
            if status in ("CONFIRMED_FOUND", "STILL_FOUND") and valid_slots:
                best = max(valid_slots, key=lambda s: s.area)
                area_frac = best.area / frame_area
                y2_frac = best.y2 / frame_h

                ready_area = area_frac >= self.vcfg.PARK_READY_AREA
                ready_y2 = y2_frac >= self.vcfg.PARK_READY_Y2
                logger.info(
                    f"Valid slot: area={area_frac:.1%} "
                    f"(>={self.vcfg.PARK_READY_AREA:.0%}? {'YES' if ready_area else 'NO'}) "
                    f"y2={y2_frac:.1%} "
                    f"(>={self.vcfg.PARK_READY_Y2:.0%}? {'YES' if ready_y2 else 'NO'})"
                )

                if ready_area or ready_y2:
                    # Close enough — trigger parking
                    logger.info(
                        f"Parking triggered! slot area={area_frac:.2%}, "
                        f"y2={y2_frac:.2%}, cx={best.center_x:.1f}"
                    )
                    logger.info("Panning centre to coast forward alongside slot")
                    self._send_nav(NavCommand(command="PAN_CAMERA_CENTER"))
                    self._approach_phase = "park_pan_centre"
                    self._approach_phase_time = now
                    return

            # Time to go back to lane tracking
            if elapsed >= self.vcfg.APPROACH_PAN_RIGHT_S:
                logger.debug("Approach: panning center — waiting for camera to settle")
                self._send_nav(NavCommand(command="PAN_CAMERA_CENTER"))
                self._approach_phase = "settling"
                self._approach_phase_time = now

        elif self._approach_phase == "settling":
            # Camera commanded to center — stay stopped until settled
            self._send_nav(NavCommand(command="STOP"))
            if elapsed >= self.vcfg.APPROACH_PAN_CENTER_SETTLE_S:
                logger.debug("Approach: camera settled — resuming lane tracking")
                self._send_nav(NavCommand(command="RESUME_LANE_TRACKING"))
                self._approach_phase = "center"
                self._approach_phase_time = now

        elif self._approach_phase == "center":
            # Pi is lane tracking forward. After duration, trigger bang-bang correction.
            if elapsed >= self.vcfg.APPROACH_PAN_CENTER_S:
                logger.debug("Approach: sending bang-bang correction")
                self._send_nav(NavCommand(command="BANG_BANG_CORRECT"))
                self._bangbang_motor_seen = False
                self._approach_phase = "bangbang"
                self._approach_phase_time = now

        elif self._approach_phase == "bangbang":
            # Wait for AnC bang-bang correction to finish (two-phase)
            bb = self._bangbang_done(elapsed)
            if bb == "complete":
                logger.debug("Approach: bang-bang complete (motors stopped)")
                self._send_nav(NavCommand(command="STOP"))
                self._approach_phase = "stopping"
                self._approach_phase_time = now
            elif bb == "timeout":
                logger.warning("Approach: bang-bang timeout — forcing stop")
                self._send_nav(NavCommand(command="STOP"))
                self._approach_phase = "stopping"
                self._approach_phase_time = now

        elif self._approach_phase == "stopping":
            # Settled — pan right to scan again
            self._send_nav(NavCommand(command="STOP"))
            if elapsed >= self.vcfg.APPROACH_STOP_S:
                logger.debug("Approach: panning right to scan for slot")
                self._send_nav(NavCommand(command="PAN_CAMERA_RIGHT"))
                self._approach_phase = "right"
                self._approach_phase_time = now

        # ══════════════════════════════════════════════════════════
        # PARKING — coast forward to get alongside
        # ══════════════════════════════════════════════════════════

        elif self._approach_phase == "park_pan_centre":
            # Wait for camera to settle at centre before driving
            self._send_nav(NavCommand(command="STOP"))
            if elapsed >= self.vcfg.PARK_PAN_SETTLE_S:
                logger.info("Park: camera centred — lane tracking forward to get alongside")
                self._send_nav(NavCommand(command="RESUME_LANE_TRACKING"))
                self._approach_phase = "park_coast"
                self._approach_phase_time = now

        elif self._approach_phase == "park_coast":
            # Lane tracking forward for fixed duration
            if elapsed >= self.vcfg.PARK_COAST_S:
                logger.info("Park: coast complete — stopping")
                self._send_nav(NavCommand(command="STOP"))
                self._approach_phase = "park_coast_stop"
                self._approach_phase_time = now

        elif self._approach_phase == "park_coast_stop":
            # Wait for robot to settle
            self._send_nav(NavCommand(command="STOP"))
            if elapsed >= self.vcfg.PARK_STOP_S:
                logger.info("Park: sending bang-bang correction after coast")
                self._send_nav(NavCommand(command="BANG_BANG_CORRECT"))
                self._bangbang_motor_seen = False
                self._approach_phase = "park_coast_bb"
                self._approach_phase_time = now

        elif self._approach_phase == "park_coast_bb":
            # Wait for bang-bang to finish (two-phase)
            bb = self._bangbang_done(elapsed)
            if bb == "complete":
                logger.info("Park: bang-bang complete — turning right to face slot")
                self._send_nav(NavCommand(command="STOP"))
                self._send_nav(NavCommand(command="TURN_RIGHT_90"))
                self._approach_phase = "park_face_slot"
                self._approach_phase_time = now
            elif bb == "timeout":
                logger.warning("Park: bang-bang timeout — turning anyway")
                self._send_nav(NavCommand(command="STOP"))
                self._send_nav(NavCommand(command="TURN_RIGHT_90"))
                self._approach_phase = "park_face_slot"
                self._approach_phase_time = now

        elif self._approach_phase == "park_face_slot":
            # Wait for turn to complete
            if elapsed >= self.vcfg.PARK_TURN_S:
                logger.info("Park: facing slot — checking alignment")
                self._send_nav(NavCommand(command="STOP"))
                self._park_check_reversed = False
                self._approach_phase = "park_check"
                self._approach_phase_time = now

        # ══════════════════════════════════════════════════════════
        # PARKING — alignment check
        # ══════════════════════════════════════════════════════════

        elif self._approach_phase == "park_check":
            # Camera is centre, robot faces slot. Check P sign vs frame centre.
            self._send_nav(NavCommand(command="STOP"))

            detections = parse_detections(result, self.class_names, self.vcfg.CONF_THRES)
            frame_mid_x = self.vcfg.IMG_SIZE / 2
            p_signs = [d for d in detections if d.class_name == self.pcfg.CLASS_P_SIGN]

            if p_signs:
                sign = max(p_signs, key=lambda s: s.area)
                offset_px = sign.center_x - frame_mid_x

                logger.info(
                    f"Park check: sign.cx={sign.center_x:.1f}, "
                    f"frame_mid={frame_mid_x:.1f}, offset={offset_px:.1f}px"
                )

                if abs(offset_px) <= self.vcfg.PARK_ALIGN_TOLERANCE_PX:
                    # Aligned — drive into slot
                    logger.info("Park: ALIGNED — driving into slot")
                    self._send_nav(NavCommand(command="DRIVE_FORWARD"))
                    self._approach_phase = "park_enter"
                    self._approach_phase_time = now
                else:
                    # Not aligned — need shimmy adjustment
                    # Positive offset = sign right of centre = robot needs to move forward
                    # Negative offset = sign left of centre = robot needs to move backward
                    self._park_offset_px = offset_px
                    self._park_offset_dir = 1 if offset_px > 0 else -1
                    logger.info(
                        f"Park: NOT aligned (offset={offset_px:.1f}px) — "
                        f"adjusting {'forward' if self._park_offset_dir > 0 else 'reverse'}"
                    )
                    self._send_nav(NavCommand(command="TURN_LEFT_90"))
                    self._approach_phase = "adjust_back"
                    self._approach_phase_time = now
            elif elapsed >= self.vcfg.PARK_CHECK_TIMEOUT_S:
                if not self._park_check_reversed:
                    # First timeout — reverse briefly to bring P sign into frame
                    logger.warning("Park check: P sign not visible — reversing to reframe")
                    self._park_check_reversed = True
                    self._send_nav(NavCommand(command="DRIVE_REVERSE"))
                    self._approach_phase = "park_check_reverse"
                    self._approach_phase_time = now
                else:
                    # Already reversed once — drive in blind
                    logger.warning("Park check: still no P sign after reverse — driving in")
                    self._send_nav(NavCommand(command="DRIVE_FORWARD"))
                    self._approach_phase = "park_enter"
                    self._approach_phase_time = now
            else:
                logger.debug(
                    f"Park check: waiting for P sign"
                )

        elif self._approach_phase == "park_check_reverse":
            # Reversing briefly to bring slot back into frame
            if elapsed >= self.vcfg.PARK_CHECK_REVERSE_S:
                logger.info("Park check: reverse done — re-checking alignment")
                self._send_nav(NavCommand(command="STOP"))
                self._approach_phase = "park_check"
                self._approach_phase_time = now

        # ══════════════════════════════════════════════════════════
        # PARKING — aligned, enter slot
        # ══════════════════════════════════════════════════════════

        elif self._approach_phase == "park_enter":
            # Drive into slot guided by P sign position
            detections = parse_detections(result, self.class_names, self.vcfg.CONF_THRES)
            p_signs = [d for d in detections if d.class_name == self.pcfg.CLASS_P_SIGN]
            frame_mid_x = self.vcfg.IMG_SIZE / 2
            frame_h = self.vcfg.IMG_SIZE

            if p_signs:
                sign = max(p_signs, key=lambda s: s.area)
                offset_x = sign.center_x - frame_mid_x

                # Check if P sign is close enough (robot is in the slot)
                if elapsed >= self.vcfg.PARK_ENTER_MIN_S and sign.y2 / frame_h >= self.vcfg.PARK_ENTER_CLOSE_Y:
                    logger.info(
                        f"Park: P sign close (y2={sign.y2/frame_h:.2%}) — in slot"
                    )
                    self._send_nav(NavCommand(command="STOP"))
                    self._approach_phase = "park_done"
                    self._approach_phase_time = now
                elif offset_x > self.vcfg.PARK_ENTER_DEADBAND_PX:
                    logger.debug(f"Park enter: P sign right of centre ({offset_x:+.0f}px) — steering right")
                    self._send_nav(NavCommand(command="ALIGN_RIGHT"))
                elif offset_x < -self.vcfg.PARK_ENTER_DEADBAND_PX:
                    logger.debug(f"Park enter: P sign left of centre ({offset_x:+.0f}px) — steering left")
                    self._send_nav(NavCommand(command="ALIGN_LEFT"))
                else:
                    logger.debug(f"Park enter: P sign centred ({offset_x:+.0f}px) — driving straight")
                    self._send_nav(NavCommand(command="DRIVE_FORWARD"))
            else:
                # No P sign visible — keep driving straight, rely on timeout
                logger.debug("Park enter: no P sign — driving straight")
                self._send_nav(NavCommand(command="DRIVE_FORWARD"))

            if elapsed >= self.vcfg.PARK_ENTER_S:
                logger.warning("Park: enter timeout — stopping")
                self._send_nav(NavCommand(command="STOP"))
                self._approach_phase = "park_done"
                self._approach_phase_time = now

        elif self._approach_phase == "park_done":
            # Parked, settle before rotating
            self._send_nav(NavCommand(command="STOP"))
            if elapsed >= self.vcfg.PARK_DONE_S:
                logger.info("Park: PARKED — first 90° turn")
                self._send_nav(NavCommand(command="TURN_RIGHT_90"))
                self._approach_phase = "park_rotate_1"
                self._approach_phase_time = now

        elif self._approach_phase == "park_rotate_1":
            # Wait for first 90° turn
            if elapsed >= self.vcfg.PARK_TURN_S:
                logger.info("Park: first 90° done — second 90° turn")
                self._send_nav(NavCommand(command="STOP"))
                self._send_nav(NavCommand(command="TURN_RIGHT_90"))
                self._approach_phase = "park_rotate_2"
                self._approach_phase_time = now

        elif self._approach_phase == "park_rotate_2":
            # Wait for second 90° turn
            if elapsed >= self.vcfg.PARK_TURN_S:
                logger.info("Park: 180° rotation complete — driving out of slot")
                self._send_nav(NavCommand(command="STOP"))
                self._send_nav(NavCommand(command="DRIVE_FORWARD"))
                self._approach_phase = "park_exit_fwd"
                self._approach_phase_time = now

        elif self._approach_phase == "park_exit_fwd":
            # Driving forward out of slot
            if elapsed >= self.vcfg.PARK_EXIT_FWD_S:
                logger.info("Park: out of slot — turning right into lane")
                self._send_nav(NavCommand(command="STOP"))
                self._send_nav(NavCommand(command="TURN_RIGHT_90"))
                self._approach_phase = "park_exit_turn"
                self._approach_phase_time = now

        elif self._approach_phase == "park_exit_turn":
            # Wait for turn into lane
            if elapsed >= self.vcfg.PARK_TURN_S:
                logger.info("Park: in lane — bang-bang correction")
                self._send_nav(NavCommand(command="STOP"))
                self._send_nav(NavCommand(command="BANG_BANG_CORRECT"))
                self._bangbang_motor_seen = False
                self._approach_phase = "park_exit_bb"
                self._approach_phase_time = now

        elif self._approach_phase == "park_exit_bb":
            # Wait for bang-bang to finish, then resume lane following (two-phase)
            bb = self._bangbang_done(elapsed)
            if bb == "complete":
                logger.info("Park: bang-bang complete — resuming lane following")
                self._send_nav(NavCommand(command="STOP"))
                self._send_nav(NavCommand(command="RESUME_LANE_TRACKING"))
                self._set_state(OverallState.LANE_FOLLOWING)
            elif bb == "timeout":
                logger.warning("Park: exit bang-bang timeout — resuming lane following anyway")
                self._send_nav(NavCommand(command="STOP"))
                self._send_nav(NavCommand(command="RESUME_LANE_TRACKING"))
                self._set_state(OverallState.LANE_FOLLOWING)

        # ══════════════════════════════════════════════════════════
        # SHIMMY ADJUSTMENT — not aligned, correct position
        # ══════════════════════════════════════════════════════════

        elif self._approach_phase == "adjust_back":
            # Turning left 90° back to lane
            if elapsed >= self.vcfg.PARK_TURN_S:
                logger.info("Adjust: back in lane — bang-bang correction")
                self._send_nav(NavCommand(command="STOP"))
                self._send_nav(NavCommand(command="BANG_BANG_CORRECT"))
                self._bangbang_motor_seen = False
                self._approach_phase = "adjust_bb1"
                self._approach_phase_time = now

        elif self._approach_phase == "adjust_bb1":
            # Wait for bang-bang after rotating back to lane (two-phase)
            bb = self._bangbang_done(elapsed)
            if bb in ("complete", "timeout"):
                if bb == "timeout":
                    logger.warning("Adjust: bb1 timeout — driving anyway")
                else:
                    logger.info("Adjust: bang-bang complete — driving to correct offset")
                self._send_nav(NavCommand(command="STOP"))
                drive_duration = abs(self._park_offset_px) * self.vcfg.ADJUST_PX_TO_S
                self._adjust_drive_duration = max(0.1, min(drive_duration, 0.5))
                if self._park_offset_dir > 0:
                    logger.info(f"Adjust: driving FORWARD for {self._adjust_drive_duration:.2f}s")
                    self._send_nav(NavCommand(command="RESUME_LANE_TRACKING"))
                else:
                    logger.info(f"Adjust: driving REVERSE for {self._adjust_drive_duration:.2f}s")
                    self._send_nav(NavCommand(command="DRIVE_REVERSE"))
                self._approach_phase = "adjust_drive"
                self._approach_phase_time = now

        elif self._approach_phase == "adjust_drive":
            # Driving forward or reverse to correct lateral offset
            if elapsed >= self._adjust_drive_duration:
                logger.info("Adjust: drive complete — stopping")
                self._send_nav(NavCommand(command="STOP"))
                self._approach_phase = "adjust_drive_stop"
                self._approach_phase_time = now

        elif self._approach_phase == "adjust_drive_stop":
            # Settle after adjustment drive
            self._send_nav(NavCommand(command="STOP"))
            if elapsed >= self.vcfg.PARK_STOP_S:
                logger.info("Adjust: bang-bang correction after drive")
                self._send_nav(NavCommand(command="BANG_BANG_CORRECT"))
                self._bangbang_motor_seen = False
                self._approach_phase = "adjust_bb2"
                self._approach_phase_time = now

        elif self._approach_phase == "adjust_bb2":
            # Wait for bang-bang after adjustment drive (two-phase)
            bb = self._bangbang_done(elapsed)
            if bb == "complete":
                logger.info("Adjust: bang-bang complete — turning right to face slot")
                self._send_nav(NavCommand(command="STOP"))
                self._send_nav(NavCommand(command="TURN_RIGHT_90"))
                self._approach_phase = "adjust_face_slot"
                self._approach_phase_time = now
            elif bb == "timeout":
                logger.warning("Adjust: bb2 timeout — turning anyway")
                self._send_nav(NavCommand(command="STOP"))
                self._send_nav(NavCommand(command="TURN_RIGHT_90"))
                self._approach_phase = "adjust_face_slot"
                self._approach_phase_time = now

        elif self._approach_phase == "adjust_face_slot":
            # Wait for turn to complete, then re-check alignment
            if elapsed >= self.vcfg.PARK_TURN_S:
                logger.info("Adjust: facing slot again — re-checking alignment")
                self._send_nav(NavCommand(command="STOP"))
                self._park_check_reversed = False
                self._approach_phase = "park_check"
                self._approach_phase_time = now

    # ----------------------------------------------------------
    # Bang-bang two-phase completion
    # ----------------------------------------------------------

    def _bangbang_done(self, elapsed: float):
        """
        Two-phase check: wait for motors to START moving, then wait for
        them to STOP.  Returns "complete", "timeout", or None.
        """
        if elapsed < self.vcfg.BANGBANG_STARTUP_S:
            return None                          # startup buffer

        if not self._bangbang_motor_seen:
            if not self.motor_monitor.both_stopped():
                self._bangbang_motor_seen = True  # correction started
                return None
            # Motors still at zero — haven't started yet
            if elapsed >= self.vcfg.BANGBANG_TIMEOUT_S:
                return "timeout"
            return None

        # Correction started — wait for motors to return to zero
        if self.motor_monitor.both_stopped():
            return "complete"
        if elapsed >= self.vcfg.BANGBANG_TIMEOUT_S:
            return "timeout"
        return None

    # ----------------------------------------------------------
    # Nav Command Publishing
    # ----------------------------------------------------------

    _last_nav_cmd: str = ""

    def _send_nav(self, cmd: NavCommand):
        """Send a navigation command to AnC via Zenoh (recipe of topic+value pairs)."""
        if cmd.command != self._last_nav_cmd:
            logger.info(f"NAV >> {cmd.command}  {cmd.to_dict()['topics']}")
            self._last_nav_cmd = cmd.command
        else:
            logger.debug(f"NAV >> {cmd.command} (repeat)")
        cmd.publish_all(self.zenoh)

    def _publish_detections(self, detections: List[Detection]):
        """Publish detection summary to AnC."""
        frame_area = self.vcfg.IMG_SIZE ** 2
        summary = {
            "frame_id": self.frame_id,
            "timestamp": time.time(),
            "state": self.state.name,
            "count": len(detections),
            "objects": [
                {
                    "class": d.class_name,
                    "conf": round(d.confidence, 3),
                    "cx": round(d.center_x, 1),
                    "cy": round(d.center_y, 1),
                    "w": round(d.width, 1),
                    "h": round(d.height, 1),
                    "area_pct": round(d.area / frame_area, 4),
                }
                for d in detections
            ],
        }
        self.zenoh.publish(self.vcfg.ZENOH_TOPIC_DETECTIONS, summary)

    # ----------------------------------------------------------
    # State Transition
    # ----------------------------------------------------------

    def _set_state(self, new_state: OverallState):
        logger.info(f"State: {self.state.name} → {new_state.name}")
        self.state = new_state
        if self.vcfg.DEBUG_PUBLISH:
            payload = {"state": new_state.name, "timestamp": time.time()}
            if new_state == OverallState.APPROACH_PARKING:
                payload["phase"] = self._approach_phase
            self.zenoh.publish(self.vcfg.ZENOH_TOPIC_STATE, payload)

    # ----------------------------------------------------------
    # Initialization
    # ----------------------------------------------------------

    def _load_model(self):
        """Load YOLO model with configured device (MPS for Mac, CUDA for NVIDIA)."""
        logger.info(f"Loading model: {self.vcfg.MODEL_PATH} on device={self.vcfg.DEVICE}")
        self.model = YOLO(self.vcfg.MODEL_PATH, task=self.vcfg.TASK)

        self.class_names = self.model.names
        self.class_colors = generate_colors(len(self.class_names))
        logger.info(f"Model loaded — {len(self.class_names)} classes, device={self.vcfg.DEVICE}")

    def _connect_camera(self):
        logger.info(f"Connecting to: {self.vcfg.STREAM_URL}")
        for attempt in range(1, self.vcfg.MAX_RETRIES + 1):
            self.grabber = FrameGrabber(self.vcfg.STREAM_URL)
            if self.grabber.isOpened():
                logger.info(f"Camera connected via threaded grabber (attempt {attempt})")
                return
            self.grabber.release()
            logger.warning(f"Attempt {attempt}/{self.vcfg.MAX_RETRIES} failed")
            time.sleep(2)
        raise ConnectionError("Camera connection failed after all retries")

    def _setup_zenoh(self):
        self.zenoh.open()
        # Declare a publisher for every nav-command topic from zenoh_topics.md
        for topic in NAV_CMD_TOPICS:
            self.zenoh.declare_publisher(topic)
        if self.vcfg.DEBUG_PUBLISH:
            self.zenoh.declare_publisher(self.vcfg.ZENOH_TOPIC_STATE)
            self.zenoh.declare_publisher(self.vcfg.ZENOH_TOPIC_DETECTIONS)

        # Subscribe to motor speed feedback from AnC (for bang-bang completion)
        self.zenoh.declare_subscriber(
            MotorSpeedMonitor.TOPIC_LMTR, self.motor_monitor.on_lmtr_speed)
        self.zenoh.declare_subscriber(
            MotorSpeedMonitor.TOPIC_RMTR, self.motor_monitor.on_rmtr_speed)

        # Initialise all bstn topics to safe state (zenoh_topics.md requirement)
        logger.info("Initialising Zenoh topics to safe state")
        NavCommand(command="INIT_SAFE_STATE").publish_all(self.zenoh)

    def _setup_logging(self):
        os.makedirs(self.vcfg.LOG_DIR, exist_ok=True)
        log_path = os.path.join(self.vcfg.LOG_DIR, f"vision_{self.run_id}.jsonl")
        self.log_file = open(log_path, "w")

        meta = {
            "run_id": self.run_id,
            "start_time": time.time(),
            "model": self.vcfg.MODEL_PATH,
            "stream": self.vcfg.STREAM_URL,
            "classes": {str(k): v for k, v in self.class_names.items()},
        }
        self.log_file.write(json.dumps({"meta": meta}) + "\n")
        logger.info(f"Logging to: {log_path}")

    # ----------------------------------------------------------
    # Frame Logging
    # ----------------------------------------------------------

    def _log_frame(self, detections: List[Detection]):
        if not self.vcfg.LOG_DETECTIONS or self.log_file is None:
            return
        entry = {
            "frame_id": self.frame_id,
            "processed_id": self.processed_count,
            "timestamp": time.time(),
            "state": self.state.name,
            "phase": self._approach_phase if self.state == OverallState.APPROACH_PARKING else None,
            "detections": [
                {
                    "class": d.class_name,
                    "conf": round(d.confidence, 3),
                    "xyxy": [round(d.x1, 1), round(d.y1, 1),
                             round(d.x2, 1), round(d.y2, 1)],
                }
                for d in detections
            ],
        }
        self.log_file.write(json.dumps(entry) + "\n")
        self.log_file.flush()

    # ----------------------------------------------------------
    # Visualization
    # ----------------------------------------------------------

    def _visualize(self, frame, result, detections: List[Detection]):
        """Rich annotated overlay with slot validity, foot-points, masks, FPS."""
        annotated = frame.copy()
        h, w = annotated.shape[:2]
        frame_area = self.vcfg.IMG_SIZE ** 2

        # ── Classify all parking slots ──
        slot_status_map = classify_all_slots(detections, self.pcfg)

        # ── Draw bounding boxes, labels, slot validity tags ──
        for d in detections:
            color = self.class_colors.get(d.class_id, (0, 255, 0))
            x1, y1, x2, y2 = int(d.x1), int(d.y1), int(d.x2), int(d.y2)
            cv2.rectangle(annotated, (x1, y1), (x2, y2), color, 2)

            label = f"{d.class_name} {d.confidence:.2f}"

            # Tag parking slots: VALID / INVALID / UNKNOWN
            if d.class_name == self.pcfg.CLASS_PARKING_SLOT:
                slot_status = slot_status_map.get(id(d), "UNKNOWN")
                area_pct = d.area / frame_area * 100
                status_color = {
                    "VALID":   (0, 255, 0),
                    "INVALID": (0, 0, 255),
                    "UNKNOWN": (0, 200, 255),
                }.get(slot_status, (200, 200, 200))
                label += f" [{slot_status} {area_pct:.1f}%]"
                cv2.rectangle(annotated, (x1, y1), (x2, y2), status_color, 3)
                scx, scy = int(d.center_x), int(d.center_y)
                cv2.drawMarker(annotated, (scx, scy), status_color,
                               cv2.MARKER_TILTED_CROSS, 16, 2)

            # Draw foot-point for cones, disabled signs, P signs
            if d.class_name in (self.pcfg.CLASS_CONE,
                                self.pcfg.CLASS_DISABLED_SIGN,
                                self.pcfg.CLASS_P_SIGN):
                foot_x, foot_y = int(d.center_x), int(d.y2)
                cv2.circle(annotated, (foot_x, foot_y), 6, (0, 255, 255), -1)
                cv2.drawMarker(annotated, (foot_x, foot_y), (0, 255, 255),
                               cv2.MARKER_CROSS, 14, 2)

            (tw, th), _ = cv2.getTextSize(label, cv2.FONT_HERSHEY_SIMPLEX, 0.5, 2)
            cv2.rectangle(annotated, (x1, y1 - th - 10), (x1 + tw, y1), color, -1)
            cv2.putText(annotated, label, (x1, y1 - 5),
                        cv2.FONT_HERSHEY_SIMPLEX, 0.5, (255, 255, 255), 2)

        # ── Draw segmentation masks (parking slots only) ──
        if result.masks is not None:
            slot_cls_ids = [
                cid for cid, name in self.class_names.items()
                if name == self.vcfg.CLASS_PARKING_SLOT
            ]
            for i, mask in enumerate(result.masks.data):
                cls_id = int(result.boxes.cls[i])
                if cls_id not in slot_cls_ids:
                    continue
                mask_np = mask.cpu().numpy().astype(np.uint8)
                mask_resized = cv2.resize(mask_np, (self.vcfg.IMG_SIZE, self.vcfg.IMG_SIZE))
                color = self.class_colors.get(cls_id, (0, 255, 0))
                colored_mask = np.zeros_like(annotated)
                colored_mask[mask_resized > 0] = color
                annotated = cv2.addWeighted(annotated, 1.0, colored_mask, 0.35, 0)

        # ── Top bar (semi-transparent) ──
        overlay = annotated.copy()
        cv2.rectangle(overlay, (0, 0), (w, 70), (0, 0, 0), -1)
        cv2.addWeighted(overlay, 0.6, annotated, 0.4, 0, annotated)

        # State info — left side
        state_text = f"STATE: {self.state.name}"
        if self.state == OverallState.APPROACH_PARKING:
            state_text += f" | PHASE: {self._approach_phase}"
        cv2.putText(annotated, state_text, (10, 25),
                    cv2.FONT_HERSHEY_SIMPLEX, 0.6, (0, 200, 255), 2)

        info_text = f"Frame: {self.processed_count} | Objects: {len(detections)} | Infer: {self._infer_ms:.0f}ms"
        cv2.putText(annotated, info_text, (10, 55),
                    cv2.FONT_HERSHEY_SIMPLEX, 0.45, (200, 200, 200), 1)

        # FPS — top right
        fps_text = f"FPS: {self._fps:.1f}"
        (fw, fh), _ = cv2.getTextSize(fps_text, cv2.FONT_HERSHEY_SIMPLEX, 0.7, 2)
        cv2.putText(annotated, fps_text, (w - fw - 10, 30),
                    cv2.FONT_HERSHEY_SIMPLEX, 0.7, (0, 255, 0), 2)

        cv2.imshow("Vision Service", annotated)

    # ----------------------------------------------------------
    # Cleanup
    # ----------------------------------------------------------

    def _cleanup(self):
        logger.info("Cleaning up...")
        if self.grabber:
            self.grabber.release()
        cv2.destroyAllWindows()
        if self.log_file:
            self.log_file.close()
        self.zenoh.close()
        logger.info(f"Final state: {self.state.name}")
        logger.info(f"Frames processed: {self.processed_count}")


# ============================================================
# Entry Point
# ============================================================

if __name__ == "__main__":
    vcfg = VisionConfig()
    pcfg = ParkingConfig()

    service = VisionService(vcfg, pcfg)
    service.start()
