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
  - Detect valid parking slots → trigger state transition internally
  - Wait for prepare_to_park ACK before panning camera / starting parking SM
  - Manage the parking state machine (slot detection, alignment)
  - Send detection events and nav commands to AnC via Zenoh

Zenoh Topics (Core — always active, per zenoh_topics.md):
  palanuk/bstn/stop       — stop / resume (u8: 1=stop, 0=resume)
  palanuk/bstn/loopmode   — loop mode (u8: 0=Open, 1=Closed)
  palanuk/bstn/speed      — speed setpoint (f64)
  palanuk/bstn/steercmd   — steering command (u8: 0=Free, 1=Hard Left, 2=Hard Right)
  palanuk/bstn/drivestate — drive state (u8: 0=Rest, 1=Forward, 2=Reverse)
  palanuk/bstn/forcepan   — camera pan (u8: 0=Center, 1=Left, 2=Right)
  palanuk/itp/accelerate  — bumper acceleration (u8: 0/1, rising edge)
  anc/ack                 — acknowledgments from AnC (JSON)  [toggle via ENABLE_ACK]

  Each nav command publishes a *recipe* — a combination of the above
  topics — defined in NAV_CMD_RECIPES (parking_service.py).

Zenoh Topics (Debug — toggle via DEBUG_PUBLISH):
  anc/state             — current navigation state (JSON)
  anc/detections        — per-frame detection summary (JSON)

Overall States:
  LANE_FOLLOWING        — Pi handles lane tracking; vision watches for bumpers
  APPROACH_PARKING      — Scanning for valid slot; initialising parking SM
  PARKING               — Delegated to ParkingStateMachine
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
    ParkingStateMachine,
    ParkingConfig,
    ParkingState,
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
    PARKING = auto()
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
    VALID_PARKING_AREA_THRESHOLD: float = 0.20  # 10% of frame

    # Debounce frame counts
    BUMPER_DEBOUNCE_FRAMES: int = 3
    SLOT_DEBOUNCE_FRAMES: int = 3

    # ACK handling — set to False to skip waiting for AnC acknowledgments
    ENABLE_ACK: bool = False

    # Zenoh topics — core (always active)
    # Nav command topics are defined per-command in NAV_CMD_RECIPES (parking_service.py)
    ZENOH_TOPIC_ACK: str = "anc/ack"               # AnC → ITP: acknowledgments

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

        # Parking state machine
        self.parking_sm: Optional[ParkingStateMachine] = None

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

        # Fire-once guards
        self._parked_triggered: bool = False

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

            # Handle ACKs from AnC (if enabled)
            if self.vcfg.ENABLE_ACK:
                self._process_acks()

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
        elif self.state == OverallState.PARKING:
            self._state_parking(result)

    def _state_lane_following(self, detections: List[Detection]):
        """
        LANE_FOLLOWING: Pi is lane tracking autonomously.
        Vision watches for bumpers and parking slots.

        Bumper: sends ACCELERATE_FOR_BUMP once when bumper y2 is near
                the top of the frame (far away → accelerate early).
        Parking: sends PARKING_SLOTS_DETECTED once when a valid slot is
                 confirmed. Waits for prepare_to_park ACK before
                 transitioning.
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
            logger.info("Panning camera right for parking approach")
            self._send_nav(NavCommand(command="PAN_CAMERA_RIGHT"))
            # Camera is panned right so Pi can't lane-track — drive forward open-loop
            logger.info("Waiting 1s before DRIVE_FORWARD (camera settling)")
            time.sleep(1.0)
            logger.info("Sending DRIVE_FORWARD (camera panned, lane tracking unavailable)")
            self._send_nav(NavCommand(command="DRIVE_FORWARD"))

            if not self.vcfg.ENABLE_ACK:
                logger.info("ACK disabled — auto-transitioning to APPROACH_PARKING")
                self._set_state(OverallState.APPROACH_PARKING)
            # else: wait for prepare_to_park ACK from AnC (see _process_acks)

    def _state_approach_parking(self, result):
        """
        APPROACH_PARKING: Camera is panned right.
        Scan for a VALID parking slot. Once confirmed, initialize
        the parking state machine and transition to PARKING.
        """
        detections = parse_detections(result, self.class_names, self.vcfg.CONF_THRES)
        frame_area = self.vcfg.IMG_SIZE ** 2

        slot_status_map = classify_all_slots(detections, self.pcfg)
        valid_slots = [
            d for d in detections
            if d.class_name == self.pcfg.CLASS_PARKING_SLOT
            and slot_status_map.get(id(d), "UNKNOWN") == "VALID"
            and d.area / frame_area >= self.vcfg.VALID_PARKING_AREA_THRESHOLD
        ]

        status = self.valid_slot_tracker.update(len(valid_slots) > 0)
        if status == "CONFIRMED_FOUND":
            best = max(valid_slots, key=lambda s: s.area)
            logger.info(
                f"Valid parking slot confirmed "
                f"(area={best.area/frame_area:.2%}, "
                f"cx={best.center_x:.1f}, cy={best.center_y:.1f})"
            )
            logger.info("Initializing parking state machine")
            self.parking_sm = ParkingStateMachine(
                self.pcfg,
                on_command=self._parking_command_handler,
                enable_ack=self.vcfg.ENABLE_ACK,
            )
            self._parked_triggered = False
            self._set_state(OverallState.PARKING)

    def _state_parking(self, result):
        """
        PARKING: Delegate all frame processing to ParkingStateMachine.
        """
        parking_state = self.parking_sm.process_frame(result, self.class_names)

        if parking_state == ParkingState.PARKED and not self._parked_triggered:
            self._parked_triggered = True
            logger.info("Robot is PARKED — triggering exit after 1s")
            time.sleep(1.0)
            self.parking_sm.trigger_exit()

        elif parking_state == ParkingState.COMPLETE:
            logger.info("Parking complete")
            self._set_state(OverallState.FINISHED)

        elif parking_state == ParkingState.FAILED:
            logger.error("Parking FAILED")
            self._set_state(OverallState.ERROR)

    # ----------------------------------------------------------
    # ACK Processing from AnC
    # ----------------------------------------------------------

    def _process_acks(self):
        """Process acknowledgments from AnC (only when ENABLE_ACK is True)."""
        if not self.vcfg.ENABLE_ACK:
            return
        for msg in self.zenoh.get_acks():
            ack_type = msg.get("ack", "")
            logger.info(f"ACK from AnC: {ack_type}")

            if ack_type == "prepare_to_park" and self.state == OverallState.LANE_FOLLOWING:
                logger.info("AnC ready to park — entering approach")
                self._set_state(OverallState.APPROACH_PARKING)

            # Forward ACKs to parking state machine if active
            if self.parking_sm and self.state == OverallState.PARKING:
                self.parking_sm.on_ack(ack_type)

    # ----------------------------------------------------------
    # Nav Command Publishing
    # ----------------------------------------------------------

    def _send_nav(self, cmd: NavCommand):
        """Send a navigation command to AnC via Zenoh (recipe of topic+value pairs)."""
        logger.info(f"NAV >> {cmd.command}  {cmd.to_dict()['topics']}")
        cmd.publish_all(self.zenoh)

    def _parking_command_handler(self, cmd: NavCommand):
        """Callback from ParkingStateMachine — forward to AnC."""
        self._send_nav(cmd)

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
            self.zenoh.publish(self.vcfg.ZENOH_TOPIC_STATE, {
                "state": new_state.name,
                "timestamp": time.time(),
            })

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
        if self.vcfg.ENABLE_ACK:
            self.zenoh.declare_subscriber(self.vcfg.ZENOH_TOPIC_ACK, self.zenoh._on_ack)
        if self.vcfg.DEBUG_PUBLISH:
            self.zenoh.declare_publisher(self.vcfg.ZENOH_TOPIC_STATE)
            self.zenoh.declare_publisher(self.vcfg.ZENOH_TOPIC_DETECTIONS)

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
        if self.state == OverallState.PARKING and self.parking_sm:
            state_text += f" | PARK: {self.parking_sm.state.name}"
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