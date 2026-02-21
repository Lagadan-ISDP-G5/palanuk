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
  - Detect bumpers → notify AnC
  - Manage the parking state machine (slot detection, alignment)
  - Send detection events and nav commands to AnC via Zenoh

Zenoh Topics Published:
  anc/state             — current navigation state (JSON)
  anc/nav_command       — navigational commands for AnC (JSON)
  anc/detections        — per-frame detection summary (JSON)

Zenoh Topics Subscribed:
  anc/ack               — acknowledgments from AnC (JSON)  [toggle via ENABLE_ACK]

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
from enum import Enum, auto
from datetime import datetime
from typing import Optional, List, Dict, Any
from ultralytics import YOLO
import numpy as np

from parking_service import (
    ParkingStateMachine,
    ParkingConfig,
    ParkingState,
    NavCommand,
    parse_detections,
    Detection,
    DebounceTracker,
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
    MODEL_PATH: str = "tests/best.onnx"
    IMG_SIZE: int = 640
    CONF_THRES: float = 0.4
    DEVICE: str = "0"
    TASK: str = "segment"

    # Stream
    STREAM_URL: str = "rtsp://192.168.93.163:8554/camera"
    MAX_RETRIES: int = 5

    # Processing
    SKIP_FRAMES: int = 1
    USE_ONLY_BOXES: bool = True

    # Class names (must match YOLO model)
    CLASS_BUMPER: str = "bumper"
    CLASS_PARKING_SLOT: str = "parking_slot"
    CLASS_P_SIGN: str = "parking_signboard"
    CLASS_DISABLED_SIGN: str = "disabled person signboard"
    CLASS_CONE: str = "cone"

    # Detection thresholds (fraction of frame area)
    BUMPER_AREA_THRESHOLD: float = 0.08

    # Debounce frame counts
    BUMPER_DEBOUNCE_FRAMES: int = 3

    # ACK handling — set to False to skip waiting for AnC acknowledgments
    ENABLE_ACK: bool = False

    # Zenoh topics
    ZENOH_TOPIC_STATE: str = "anc/state"
    ZENOH_TOPIC_NAV_CMD: str = "anc/nav_command"
    ZENOH_TOPIC_DETECTIONS: str = "anc/detections"
    ZENOH_TOPIC_ACK: str = "anc/ack"

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
        """Publish dict or string to a Zenoh topic."""
        payload = json.dumps(data) if isinstance(data, dict) else str(data)

        if topic in self.publishers:
            self.publishers[topic].put(payload)
        else:
            logger.info(f"[PUB {topic}] {payload}")

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
        self.cap: Optional[cv2.VideoCapture] = None

        # Zenoh
        self.zenoh = ZenohBridge()

        # Parking state machine
        self.parking_sm: Optional[ParkingStateMachine] = None

        # Debounce trackers
        self.bumper_tracker = DebounceTracker(
            found_thresh=vcfg.BUMPER_DEBOUNCE_FRAMES, lost_thresh=3
        )

        # Logging
        self.log_file = None
        self.run_id = datetime.now().strftime("%Y-%m-%d_%H-%M-%S")

        # Stats
        self.frame_id = 0
        self.processed_count = 0

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
            ret, frame = self.cap.read()
            if not ret:
                logger.warning("Frame read failed — reconnecting...")
                self.cap.release()
                time.sleep(1)
                self.cap = cv2.VideoCapture(self.vcfg.STREAM_URL)
                continue

            self.frame_id += 1
            if self.frame_id % self.vcfg.SKIP_FRAMES != 0:
                continue

            frame_resized = cv2.resize(frame, (self.vcfg.IMG_SIZE, self.vcfg.IMG_SIZE))

            # Inference
            result = self.model.predict(
                source=frame_resized,
                imgsz=self.vcfg.IMG_SIZE,
                conf=self.vcfg.CONF_THRES,
                device=self.vcfg.DEVICE,
                verbose=False,
            )[0]

            detections = parse_detections(result, self.class_names, self.vcfg.CONF_THRES)

            # Log
            self._log_frame(detections)

            # Publish raw detections to AnC
            self._publish_detections(detections)

            # Process state machine
            self._process_state(result, detections)

            # Handle ACKs from AnC (if enabled)
            if self.vcfg.ENABLE_ACK:
                self._process_acks()

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
        Vision watches for bumpers and notifies AnC.
        """
        frame_area = self.vcfg.IMG_SIZE ** 2

        # Report bumper if seen
        bumpers = [
            d for d in detections
            if d.class_name == self.vcfg.CLASS_BUMPER
            and d.area / frame_area >= self.vcfg.BUMPER_AREA_THRESHOLD
        ]

        status = self.bumper_tracker.update(len(bumpers) > 0)
        if status == "CONFIRMED_FOUND":
            b = bumpers[0]
            logger.info(f"Bumper confirmed (area={b.area/frame_area:.4f})")
            self._send_nav(NavCommand(
                command="BUMPER_DETECTED",
                metadata={
                    "center_x": round(b.center_x, 1),
                    "center_y": round(b.center_y, 1),
                    "area_pct": round(b.area / frame_area, 4),
                },
            ))

    def _state_approach_parking(self, result):
        """
        APPROACH_PARKING: Just entered parking zone.
        Initialize parking state machine and transition to PARKING.
        """
        logger.info("Initializing parking state machine")
        self.parking_sm = ParkingStateMachine(
            self.pcfg,
            on_command=self._parking_command_handler,
        )
        self._set_state(OverallState.PARKING)

    def _state_parking(self, result):
        """
        PARKING: Delegate all frame processing to ParkingStateMachine.
        """
        parking_state = self.parking_sm.process_frame(result, self.class_names)

        if parking_state == ParkingState.PARKED:
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

            if ack_type == "parking_zone_reached" and self.state == OverallState.LANE_FOLLOWING:
                logger.info("AnC reached parking zone — entering approach")
                self._set_state(OverallState.APPROACH_PARKING)

            # Forward ACKs to parking state machine if active
            if self.parking_sm and self.state == OverallState.PARKING:
                self.parking_sm.on_ack(ack_type)

    # ----------------------------------------------------------
    # Nav Command Publishing
    # ----------------------------------------------------------

    def _send_nav(self, cmd: NavCommand):
        """Send a navigation command to AnC via Zenoh."""
        self.zenoh.publish(self.vcfg.ZENOH_TOPIC_NAV_CMD, cmd.to_dict())

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
        self.zenoh.publish(self.vcfg.ZENOH_TOPIC_STATE, {
            "state": new_state.name,
            "timestamp": time.time(),
        })

    # ----------------------------------------------------------
    # Initialization
    # ----------------------------------------------------------

    def _load_model(self):
        logger.info(f"Loading model: {self.vcfg.MODEL_PATH}")
        self.model = YOLO(self.vcfg.MODEL_PATH, task=self.vcfg.TASK)
        self.class_names = self.model.names
        self.class_colors = generate_colors(len(self.class_names))
        logger.info(f"Model loaded — {len(self.class_names)} classes: {self.class_names}")

    def _connect_camera(self):
        logger.info(f"Connecting to: {self.vcfg.STREAM_URL}")
        for attempt in range(1, self.vcfg.MAX_RETRIES + 1):
            self.cap = cv2.VideoCapture(self.vcfg.STREAM_URL)
            if self.cap.isOpened():
                logger.info(f"Camera connected (attempt {attempt})")
                self.cap.set(cv2.CAP_PROP_FRAME_WIDTH, 640)
                self.cap.set(cv2.CAP_PROP_FRAME_HEIGHT, 480)
                return
            logger.warning(f"Attempt {attempt}/{self.vcfg.MAX_RETRIES} failed")
            time.sleep(2)
        raise ConnectionError("Camera connection failed after all retries")

    def _setup_zenoh(self):
        self.zenoh.open()
        self.zenoh.declare_publisher(self.vcfg.ZENOH_TOPIC_STATE)
        self.zenoh.declare_publisher(self.vcfg.ZENOH_TOPIC_NAV_CMD)
        self.zenoh.declare_publisher(self.vcfg.ZENOH_TOPIC_DETECTIONS)
        self.zenoh.declare_subscriber(self.vcfg.ZENOH_TOPIC_ACK, self.zenoh._on_ack)

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
        annotated = frame.copy()

        if result.boxes is not None:
            for box in result.boxes:
                x1, y1, x2, y2 = map(int, box.xyxy[0])
                cls_id = int(box.cls[0])
                conf = float(box.conf[0])
                color = self.class_colors.get(cls_id, (0, 255, 0))

                cv2.rectangle(annotated, (x1, y1), (x2, y2), color, 2)

                label = f"{self.class_names[cls_id]}: {conf:.2f}"
                (lw, lh), _ = cv2.getTextSize(label, cv2.FONT_HERSHEY_SIMPLEX, 0.5, 2)
                cv2.rectangle(annotated, (x1, y1 - lh - 10), (x1 + lw, y1), color, -1)
                cv2.putText(annotated, label, (x1, y1 - 5),
                            cv2.FONT_HERSHEY_SIMPLEX, 0.5, (255, 255, 255), 2)

        # State info overlay
        state_text = f"STATE: {self.state.name}"
        if self.state == OverallState.PARKING and self.parking_sm:
            state_text += f" | PARK: {self.parking_sm.state.name}"

        cv2.putText(annotated, state_text, (10, 30),
                    cv2.FONT_HERSHEY_SIMPLEX, 0.7, (0, 0, 255), 2)

        info_text = f"Frame: {self.processed_count} | Objects: {len(detections)}"
        cv2.putText(annotated, info_text, (10, 60),
                    cv2.FONT_HERSHEY_SIMPLEX, 0.5, (0, 255, 0), 2)

        cv2.imshow("Vision Service", annotated)

    # ----------------------------------------------------------
    # Cleanup
    # ----------------------------------------------------------

    def _cleanup(self):
        logger.info("Cleaning up...")
        if self.cap:
            self.cap.release()
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