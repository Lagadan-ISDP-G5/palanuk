"""
Parking Service — Vision-Only Parking State Machine
====================================================
Runs on the base station laptop. Processes YOLO detections and
publishes navigational commands/states to AnC via Zenoh.

This module does NOT handle:
  - Motor speeds / PWM
  - Lane tracking (handled by Pi)
  - Direct hardware control

This module DOES:
  - Detect valid/invalid parking slots
  - Track when the valid slot enters/exits the camera view
  - Determine when to tell AnC to stop, turn, align, enter, etc.
  - Run a PI controller for lateral alignment (sends direction, not speed)

Commands sent to AnC:
  PAN_CAMERA_RIGHT       — servo pan camera 45° right
  PAN_CAMERA_CENTER      — servo pan camera back to center
  STOP                   — stop the robot
  TURN_RIGHT_90          — turn 90° right in place
  ALIGN_LEFT             — nudge left (with magnitude 0-1)
  ALIGN_RIGHT            — nudge right (with magnitude 0-1)
  ROTATE_180             — rotate 180° in place
  RESUME_LANE_TRACKING   — hand control back to Pi's lane tracker
"""

import time
import struct
import msgpack
import logging
import threading
from enum import Enum, auto
from dataclasses import dataclass, field
from typing import Optional, List, Dict, Any, Tuple, Union

logging.basicConfig(level=logging.INFO, format="%(asctime)s [%(name)s] %(levelname)s: %(message)s")
logger = logging.getLogger("ParkingService")


# ============================================================
# Zenoh Topic Recipes for nav commands
# ============================================================
#
# Each ITP command maps to one or more (topic, value) pairs that
# must be published together.  Topics + values follow zenoh_topics.md.
#
# Value types:
#   int   → encoded as u8  (struct "B", 1 byte)
#   float → encoded as f64 (struct "d", 8 bytes)

NAV_CMD_RECIPES: Dict[str, List[Tuple[str, Union[int, float]]]] = {
    # ── Full stop (safe state) ──
    "STOP": [
        ("palanuk/bstn/loopmode",   0),
        ("palanuk/bstn/speed",      0.0),
        ("palanuk/bstn/drivestate", 0),
        ("palanuk/bstn/steercmd",   0),
    ],

    # ── Resume lane tracking (release control to Pi) ──
    "RESUME_LANE_TRACKING": [
        ("palanuk/bstn/loopmode",       1),
        ("palanuk/bstn/speed",      0.015),
        ("palanuk/bstn/drivestate",     1)
    ],

    # ── Safe-state initialisation (all zeros, publish at startup) ──
    "INIT_SAFE_STATE": [
        ("palanuk/bstn/stop",       0),
        ("palanuk/bstn/loopmode",   0),
        ("palanuk/bstn/speed",      0.0),
        ("palanuk/bstn/drivestate", 0),
        ("palanuk/bstn/steercmd",   0),
        ("palanuk/bstn/forcepan",   0),
    ],

    # ── Bumper acceleration (ITP-specific, rising-edge trigger) ──
    "ACCELERATE_FOR_BUMP": [
        ("palanuk/itp/accelerate",   1),
    ],

    # this needs to be adjusted
    # ── Steer left  (zenoh_topics.md: steercmd=1, drivestate=1) ──
    "ALIGN_LEFT": [
        ("palanuk/bstn/loopmode",   0),
        ("palanuk/bstn/speed",      0.04),
        ("palanuk/bstn/steercmd",   1),
        ("palanuk/bstn/drivestate", 1),
    ],

    # this needs to be adjusted
    # ── Steer right (zenoh_topics.md: steercmd=1, drivestate=2) ──
    "ALIGN_RIGHT": [
        ("palanuk/bstn/loopmode",   0),
        ("palanuk/bstn/speed",      0.04),
        ("palanuk/bstn/steercmd",   1),
        ("palanuk/bstn/drivestate", 2),
    ],

    # ── Turn right 90° (same steer-right recipe; AnC measures angle) ──
    "TURN_RIGHT_90": [
        ("palanuk/bstn/loopmode",   0),
        ("palanuk/bstn/speed",      0.04),
        ("palanuk/bstn/steercmd",   1),
        ("palanuk/bstn/drivestate", 2),
    ],

    # ── Rotate 180° (same steer-right recipe; AnC measures angle) ──
    "ROTATE_180": [
        ("palanuk/bstn/loopmode",   0),
        ("palanuk/bstn/speed",      0.04),
        ("palanuk/bstn/steercmd",   1),
        ("palanuk/bstn/drivestate", 2),
    ],

    # ── Release steering (free) ──
    "STEER_FREE": [
        ("palanuk/bstn/steercmd",   0),
    ],

    # ── Drive forward ──
    "DRIVE_FORWARD": [
        ("palanuk/bstn/loopmode",   0),
        ("palanuk/bstn/speed",      0.01),
        ("palanuk/bstn/drivestate", 1),
        ("palanuk/bstn/steercmd",   0),
    ],

    # ── Drive reverse ──
    "DRIVE_REVERSE": [
        ("palanuk/bstn/loopmode",   0),
        ("palanuk/bstn/speed",      0.02),
        ("palanuk/bstn/drivestate", 2),
        ("palanuk/bstn/steercmd",   0),
    ],

    # ── Drive at rest ──
    "DRIVE_REST": [
        ("palanuk/bstn/loopmode",   0),
        ("palanuk/bstn/speed",      0.0),
        ("palanuk/bstn/drivestate", 0),
        ("palanuk/bstn/steercmd",   0),
    ],

    # ── Camera pan ──
    "PAN_CAMERA_RIGHT": [
        ("palanuk/bstn/forcepan",   2),
    ],
    "PAN_CAMERA_CENTER": [
        ("palanuk/bstn/forcepan",   0),
    ],
    "PAN_CAMERA_LEFT": [
        ("palanuk/bstn/forcepan",   1),
    ],
}

# All unique Zenoh topics ITP publishes nav commands to
NAV_CMD_TOPICS: List[str] = sorted(
    set(topic for recipe in NAV_CMD_RECIPES.values() for topic, _ in recipe)
)


# ============================================================
# Configuration
# ============================================================

class ParkingConfig:
    """All tunable parameters for the parking state machine."""

    # Detection confidence
    MIN_CONFIDENCE: float = 0.50

    # Class names (must match YOLO model)
    CLASS_PARKING_SLOT: str = "parking_slot"
    CLASS_P_SIGN: str = "parking_signboard"
    CLASS_DISABLED_SIGN: str = "disabled person signboard"
    CLASS_CONE: str = "cone"

    # Coasting — frames to wait after losing valid slot before sending STOP
    COAST_FRAMES: int = 15  # ~0.5s at 30fps

    # Inter-state delay (seconds) — pause between stop/turn/rotate transitions
    STATE_DELAY_S: float = 2.0

    # Debounce
    LOST_DEBOUNCE_FRAMES: int = 5
    FOUND_DEBOUNCE_FRAMES: int = 3

    # Alignment PI controller
    ALIGNMENT_KP: float = 0.006
    ALIGNMENT_KI: float = 0.0008
    ALIGNMENT_TOLERANCE_PX: int = 20  # ±pixels from frame center
    ALIGNMENT_MAX_OUTPUT: float = 1.0
    ALIGNMENT_MAX_ATTEMPTS: int = 60  # max frames before giving up
    ALIGNMENT_INTEGRAL_WINDUP: float = 200.0

    # Slot detection — minimum area as fraction of frame
    SLOT_AREA_THRESHOLD: float = 0.012

    # P sign proximity — signboard hangs above the slot, so its base_y
    # won't always land inside the slot bbox.  Accept if within this many
    # pixels above the slot's top edge (y1).
    P_SIGN_PROXIMITY_PX: int = 50

    # Frame dimensions (must match model input)
    FRAME_W: int = 640
    FRAME_H: int = 640
    FRAME_AREA: int = FRAME_W * FRAME_H
    FRAME_CENTER_X: int = FRAME_W // 2


# ============================================================
# Data Structures
# ============================================================

class ParkingState(Enum):
    SCAN = auto()
    COASTING = auto()
    STOPPING = auto()        # stopped, waiting 2s before turning
    TURNING = auto()         # turning right 90°, waiting 2s before align
    ALIGN = auto()
    PARKED = auto()
    ROTATING = auto()        # rotating 180°, waiting 2s before complete
    COMPLETE = auto()
    FAILED = auto()


@dataclass
class Detection:
    """Single object detection from YOLO."""
    class_name: str
    class_id: int
    confidence: float
    x1: float
    y1: float
    x2: float
    y2: float

    @property
    def center_x(self) -> float:
        return (self.x1 + self.x2) / 2.0

    @property
    def center_y(self) -> float:
        return (self.y1 + self.y2) / 2.0

    @property
    def width(self) -> float:
        return self.x2 - self.x1

    @property
    def height(self) -> float:
        return self.y2 - self.y1

    @property
    def area(self) -> float:
        return self.width * self.height

    def contains_point(self, px: float, py: float) -> bool:
        return self.x1 <= px <= self.x2 and self.y1 <= py <= self.y2


@dataclass
class NavCommand:
    """
    Navigational command sent to AnC.
    AnC decides how to translate this into motor/servo actions.

    Over Zenoh, each command maps to a *recipe* — a list of
    (topic, value) pairs published together — defined in
    NAV_CMD_RECIPES (matching zenoh_topics.md).
    """
    command: str
    magnitude: float = 0.0  # 0-1 scale where applicable
    metadata: Dict[str, Any] = field(default_factory=dict)

    @property
    def recipe(self) -> List[Tuple[str, Union[int, float]]]:
        """Return the list of (topic, value) pairs for this command."""
        return NAV_CMD_RECIPES[self.command]

    # ── Wire helpers ──

    @staticmethod
    def _encode(value: Union[int, float]) -> bytes:
        """Encode a single value as MessagePack (must match cu-zenoh-src's rmp_serde::from_slice)."""
        return msgpack.packb(value)

    def publish_all(self, bridge) -> None:
        """Publish every (topic, value) in the recipe via *bridge*."""
        for topic, value in self.recipe:
            bridge.publish_bytes(topic, self._encode(value))

    # ── Logging ──

    def to_dict(self) -> dict:
        """Dict representation for logging / debug display."""
        d: Dict[str, Any] = {
            "command": self.command,
            "topics": {t: v for t, v in self.recipe},
        }
        if self.magnitude != 0.0:
            d["magnitude"] = round(self.magnitude, 4)
        if self.metadata:
            d["metadata"] = self.metadata
        return d


# ============================================================
# Helper: Parse YOLO result into Detection list
# ============================================================

def parse_detections(result, class_names: dict, min_conf: float = 0.5) -> List[Detection]:
    """
    Parse a single YOLO result object into a list of Detection dataclasses.
    """
    detections: List[Detection] = []
    if result.boxes is None:
        return detections
    for box in result.boxes:
        conf = float(box.conf[0])
        if conf < min_conf:
            continue
        cls_id = int(box.cls[0])
        x1, y1, x2, y2 = box.xyxy[0].tolist()
        detections.append(Detection(
            class_name=class_names.get(cls_id, f"unknown_{cls_id}"),
            class_id=cls_id,
            confidence=conf,
            x1=x1, y1=y1, x2=x2, y2=y2,
        ))
    return detections


# ============================================================
# Helper: Slot validation
# ============================================================

def _foot_point(det: Detection) -> Tuple[float, float]:
    """Return the foot-point (center_x, y2) used for spatial association."""
    return (det.center_x, det.y2)


def classify_all_slots(
    all_detections: List[Detection],
    cfg: ParkingConfig,
) -> Dict[int, str]:
    """
    Classify every parking slot by its **single nearest** object.

    Algorithm:
      1. Compute the slot's center_x.
      2. Among all non-slot objects (cones, signs), find the one with the
         smallest ``|object.center_x − slot.center_x|``.
      3. That single nearest object alone determines the slot's status:
           • cone / disabled-person sign  →  INVALID
           • P signboard (with proximity check)  →  VALID
      4. All other objects are left unassigned — their slot may be out of
         frame.
      5. If no objects exist  →  UNKNOWN.

    Each slot is assigned **1 type only** so that signs from adjacent bays
    cannot override the correct classification.

    Returns ``{ id(slot_detection) : status_string }``.
    """
    slots = [d for d in all_detections if d.class_name == cfg.CLASS_PARKING_SLOT]
    others = [
        d for d in all_detections
        if d.class_name in (cfg.CLASS_CONE, cfg.CLASS_DISABLED_SIGN, cfg.CLASS_P_SIGN)
    ]

    if not slots:
        return {}

    result: Dict[int, str] = {}

    for slot in slots:
        if not others:
            result[id(slot)] = "UNKNOWN"
            continue

        # Find the single nearest object by center_x distance to slot center
        nearest = min(others, key=lambda o: abs(o.center_x - slot.center_x))

        if nearest.class_name in (cfg.CLASS_DISABLED_SIGN, cfg.CLASS_CONE):
            result[id(slot)] = "INVALID"

        elif nearest.class_name == cfg.CLASS_P_SIGN:
            sign_cx = nearest.center_x
            sign_base_y = nearest.y2
            in_x = (slot.x1 - cfg.P_SIGN_PROXIMITY_PX) <= sign_cx <= (slot.x2 + cfg.P_SIGN_PROXIMITY_PX)
            in_y = (slot.y1 - cfg.P_SIGN_PROXIMITY_PX) <= sign_base_y <= slot.y2
            if in_x and in_y:
                result[id(slot)] = "VALID"
            else:
                result[id(slot)] = "UNKNOWN"
        else:
            result[id(slot)] = "UNKNOWN"

    return result


def classify_slot(slot: Detection, all_detections: List[Detection], cfg: ParkingConfig) -> str:
    """
    Backward-compatible wrapper — classifies a single slot using the
    batch function so that nearest-slot assignment is still respected.
    """
    status_map = classify_all_slots(all_detections, cfg)
    return status_map.get(id(slot), "UNKNOWN")


# ============================================================
# Debounce Tracker
# ============================================================

class DebounceTracker:
    """
    Track whether a condition is stably true or false across frames.

    Fire-once semantics:
      CONFIRMED_FOUND  — returned exactly ONCE when threshold is first met
      STILL_FOUND      — returned on every subsequent frame while still detected
      CONFIRMED_LOST   — returned exactly ONCE after lost_thresh consecutive misses
    """

    def __init__(self, found_thresh: int = 3, lost_thresh: int = 5):
        self.found_thresh = found_thresh
        self.lost_thresh = lost_thresh
        self.consecutive_found = 0
        self.consecutive_lost = 0
        self.was_confirmed = False
        self._found_fired = False  # fire-once guard

    def update(self, detected: bool) -> str:
        if detected:
            self.consecutive_found += 1
            self.consecutive_lost = 0
            if self.consecutive_found >= self.found_thresh:
                self.was_confirmed = True
                if not self._found_fired:
                    self._found_fired = True
                    return "CONFIRMED_FOUND"       # fires only ONCE
                return "STILL_FOUND"               # subsequent frames
            return "TRACKING"
        else:
            self.consecutive_lost += 1
            self.consecutive_found = 0
            if self.was_confirmed and self.consecutive_lost >= self.lost_thresh:
                self._found_fired = False          # reset so next detection can fire again
                self.was_confirmed = False
                return "CONFIRMED_LOST"
            if self.was_confirmed:
                return "UNCERTAIN"
            return "NOT_FOUND"

    def reset(self):
        self.consecutive_found = 0
        self.consecutive_lost = 0
        self.was_confirmed = False
        self._found_fired = False


# ============================================================
# PI Controller (outputs magnitude + direction, NOT motor speeds)
# ============================================================

class PIController:
    """Simple PI controller with anti-windup. Outputs normalized correction."""

    def __init__(self, kp: float, ki: float, max_integral: float, max_output: float):
        self.kp = kp
        self.ki = ki
        self.max_integral = max_integral
        self.max_output = max_output
        self.integral = 0.0

    def compute(self, error: float, dt: float) -> float:
        self.integral += error * dt
        self.integral = max(-self.max_integral, min(self.max_integral, self.integral))

        output = self.kp * error + self.ki * self.integral
        output = max(-self.max_output, min(self.max_output, output))
        return output

    def reset(self):
        self.integral = 0.0


# ============================================================
# PARKING STATE MACHINE
# ============================================================

class ParkingStateMachine:
    """
    Vision-only parking state machine.

    Call process_frame() each frame. It emits NavCommand objects
    via the on_command callback. AnC handles the actual execution.

    State flow:
      SCAN → COASTING → STOPPING (2s) → TURNING (2s) → ALIGN
           → PARKED → ROTATING (2s) → COMPLETE
    """

    def __init__(self, cfg: ParkingConfig, on_command=None):
        self.cfg = cfg
        self.state = ParkingState.SCAN
        self.on_command = on_command or (lambda cmd: None)

        # Debounce for valid slot
        self.slot_tracker = DebounceTracker(
            found_thresh=cfg.FOUND_DEBOUNCE_FRAMES,
            lost_thresh=cfg.LOST_DEBOUNCE_FRAMES,
        )

        # Coasting frame counter
        self._coast_frames = 0

        # Timer for inter-state delays (STOPPING, TURNING, ROTATING)
        self._state_timer: Optional[float] = None

        # Alignment
        self.pi_controller = PIController(
            kp=cfg.ALIGNMENT_KP,
            ki=cfg.ALIGNMENT_KI,
            max_integral=cfg.ALIGNMENT_INTEGRAL_WINDUP,
            max_output=cfg.ALIGNMENT_MAX_OUTPUT,
        )
        self._align_attempts = 0
        self._last_align_time: Optional[float] = None

        # Last known valid slot info (for logging / debug)
        self.last_valid_slot: Optional[Detection] = None

        logger.info("ParkingStateMachine created — state=SCAN")

    # ----------------------------------------------------------
    # Public API
    # ----------------------------------------------------------

    def process_frame(self, result, class_names: dict) -> ParkingState:
        """
        Process one YOLO result. Returns current state after processing.
        """
        detections = parse_detections(result, class_names, self.cfg.MIN_CONFIDENCE)

        if self.state == ParkingState.SCAN:
            self._handle_scan(detections)
        elif self.state == ParkingState.COASTING:
            self._handle_coasting(detections)
        elif self.state == ParkingState.STOPPING:
            self._handle_timer_state(
                next_state=ParkingState.TURNING,
                on_expire=self._start_turning,
            )
        elif self.state == ParkingState.TURNING:
            self._handle_timer_state(
                next_state=ParkingState.ALIGN,
                on_expire=self._start_align,
            )
        elif self.state == ParkingState.ALIGN:
            self._handle_align(detections)
        elif self.state == ParkingState.ROTATING:
            self._handle_timer_state(
                next_state=ParkingState.COMPLETE,
                on_expire=self._start_complete,
            )

        return self.state

    def trigger_exit(self):
        """Call this to initiate parking exit sequence."""
        if self.state == ParkingState.PARKED:
            logger.info("Triggering exit — rotate 180°")
            self._emit(NavCommand(command="ROTATE_180"))
            self._state_timer = time.time()
            self._transition(ParkingState.ROTATING)

    # ----------------------------------------------------------
    # State Handlers
    # ----------------------------------------------------------

    def _handle_scan(self, detections: List[Detection]):
        """
        SCAN: Camera panned right. Robot driving forward (lane tracking on Pi).
        Look for valid parking slot. Once confirmed found and then lost → COASTING.
        """
        slots = [d for d in detections if d.class_name == self.cfg.CLASS_PARKING_SLOT]

        found_valid = False
        for slot in slots:
            slot_pct = slot.area / self.cfg.FRAME_AREA
            if slot_pct < self.cfg.SLOT_AREA_THRESHOLD:
                continue

            classification = classify_slot(slot, detections, self.cfg)
            if classification == "VALID":
                found_valid = True
                self.last_valid_slot = slot
                logger.debug(f"Valid slot: center=({slot.center_x:.0f},{slot.center_y:.0f}) "
                             f"area={slot_pct:.2%}")
                break
            elif classification == "INVALID":
                logger.debug(f"Invalid slot skipped: area={slot_pct:.2%}")

        status = self.slot_tracker.update(found_valid)

        if status == "CONFIRMED_LOST":
            logger.info("Valid slot lost from view — starting coast countdown")
            self._coast_frames = 0
            self._transition(ParkingState.COASTING)

    def _handle_coasting(self, detections: List[Detection]):
        """
        COASTING: Valid slot disappeared from camera. Count frames, then tell AnC to stop.
        If slot reappears, go back to SCAN.
        """
        # Check if slot reappeared
        slots = [d for d in detections if d.class_name == self.cfg.CLASS_PARKING_SLOT]
        for slot in slots:
            slot_pct = slot.area / self.cfg.FRAME_AREA
            if slot_pct < self.cfg.SLOT_AREA_THRESHOLD:
                continue
            if classify_slot(slot, detections, self.cfg) == "VALID":
                logger.info("Valid slot re-detected during coast — back to SCAN")
                self.slot_tracker.reset()
                self._transition(ParkingState.SCAN)
                return

        self._coast_frames += 1
        if self._coast_frames >= self.cfg.COAST_FRAMES:
            logger.info(f"Coast complete ({self._coast_frames} frames) — sending STOP")
            self._emit(NavCommand(command="STOP"))
            self._state_timer = time.time()
            self._transition(ParkingState.STOPPING)

    def _handle_align(self, detections: List[Detection]):
        """
        ALIGN: Robot has turned 90° and faces the slot.
        Use PI controller on the slot's center_x vs frame center_x.
        Send ALIGN_LEFT / ALIGN_RIGHT with magnitude.
        """
        now = time.time()
        dt = now - self._last_align_time if self._last_align_time else 0.033
        self._last_align_time = now

        # Find the parking slot or P sign
        target: Optional[Detection] = None

        slots = [d for d in detections if d.class_name == self.cfg.CLASS_PARKING_SLOT]
        p_signs = [d for d in detections if d.class_name == self.cfg.CLASS_P_SIGN]

        if slots:
            target = max(slots, key=lambda s: s.area)
        elif p_signs:
            target = max(p_signs, key=lambda s: s.area)

        if target is None:
            self._align_attempts += 1
            logger.warning(f"Align: no target visible (attempt {self._align_attempts})")
            if self._align_attempts >= self.cfg.ALIGNMENT_MAX_ATTEMPTS:
                logger.error("Alignment failed — no target after max attempts")
                self._emit(NavCommand(command="STOP"))
                self._transition(ParkingState.FAILED)
            return

        self._align_attempts += 1
        error_px = target.center_x - self.cfg.FRAME_CENTER_X

        # Check if aligned
        if abs(error_px) <= self.cfg.ALIGNMENT_TOLERANCE_PX:
            logger.info(f"Aligned! error={error_px:.1f}px — driving forward into slot")
            self._emit(NavCommand(command="STOP"))
            self.pi_controller.reset()
            self._enter_slot_timed()
            return

        correction = self.pi_controller.compute(error_px, dt)

        # Positive error = slot is right of center → need to move right
        if correction > 0:
            self._emit(NavCommand(
                command="ALIGN_RIGHT",
                magnitude=abs(correction),
                metadata={"error_px": round(error_px, 1)},
            ))
        else:
            self._emit(NavCommand(
                command="ALIGN_LEFT",
                magnitude=abs(correction),
                metadata={"error_px": round(error_px, 1)},
            ))

        if self._align_attempts >= self.cfg.ALIGNMENT_MAX_ATTEMPTS:
            logger.warning("Max align attempts — entering slot anyway")
            self._emit(NavCommand(command="STOP"))
            self.pi_controller.reset()
            self._enter_slot_timed()

    # ----------------------------------------------------------
    # Enter slot (timed drive forward)
    # ----------------------------------------------------------

    def _enter_slot_timed(self, duration: float = 1.0):
        """Drive forward for *duration* seconds then stop and transition to PARKED."""
        logger.info(f"Entering slot — DRIVE_FORWARD for {duration}s")
        self._emit(NavCommand(command="DRIVE_FORWARD"))

        def _finish_enter():
            logger.info("Enter slot duration elapsed — sending STOP")
            self._emit(NavCommand(command="STOP"))
            self._transition(ParkingState.PARKED)

        threading.Timer(duration, _finish_enter).start()

    # ----------------------------------------------------------
    # Utilities
    # ----------------------------------------------------------

    def _handle_timer_state(self, next_state: ParkingState, on_expire):
        """Wait for STATE_DELAY_S seconds, then call on_expire and transition."""
        elapsed = time.time() - self._state_timer
        if elapsed >= self.cfg.STATE_DELAY_S:
            on_expire()
            self._transition(next_state)

    def _start_turning(self):
        """Called when STOPPING delay expires — send turn commands."""
        logger.info("Stop delay elapsed — panning center and turning right 90°")
        self._emit(NavCommand(command="PAN_CAMERA_CENTER"))
        self._emit(NavCommand(command="TURN_RIGHT_90"))
        self._state_timer = time.time()

    def _start_align(self):
        """Called when TURNING delay expires — begin alignment."""
        logger.info("Turn delay elapsed — starting alignment")
        self._emit(NavCommand(command="STOP"))
        self.pi_controller.reset()
        self._align_attempts = 0
        self._last_align_time = time.time()

    def _start_complete(self):
        """Called when ROTATING delay expires — resume lane tracking."""
        logger.info("Rotate delay elapsed — parking COMPLETE")
        self._emit(NavCommand(command="RESUME_LANE_TRACKING"))

    def _transition(self, new_state: ParkingState):
        logger.info(f"Parking: {self.state.name} → {new_state.name}")
        self.state = new_state

    def _emit(self, cmd: NavCommand):
        logger.info(f"  >> NAV: {cmd.to_dict()}")
        self.on_command(cmd)
