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
  ENTER_SLOT             — drive straight forward into slot
  ROTATE_180             — rotate 180° in place
  RESUME_LANE_TRACKING   — hand control back to Pi's lane tracker
"""

import math
import time
import logging
from enum import Enum, auto
from dataclasses import dataclass, field
from typing import Optional, List, Dict, Any, Tuple

logging.basicConfig(level=logging.INFO, format="%(asctime)s [%(name)s] %(levelname)s: %(message)s")
logger = logging.getLogger("ParkingService")


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
    SLOT_AREA_THRESHOLD: float = 0.02

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
    WAIT_STOP_ACK = auto()
    WAIT_TURN_ACK = auto()
    ALIGN = auto()
    WAIT_ENTER_ACK = auto()
    PARKED = auto()
    WAIT_ROTATE_ACK = auto()
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
    """
    command: str
    magnitude: float = 0.0  # 0-1 scale where applicable
    metadata: Dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> dict:
        d = {"command": self.command}
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


def _dist_to_slot(det: Detection, slot: Detection, cfg: ParkingConfig) -> float:
    """
    Distance from an object's foot-point to a slot's center.
    For P signs, the foot-point (y2) may be above the slot, so we
    only measure horizontal + vertical proximity to the slot top.
    """
    fx, fy = _foot_point(det)
    sx = slot.center_x
    sy = slot.center_y
    return math.hypot(fx - sx, fy - sy)


def classify_all_slots(
    all_detections: List[Detection],
    cfg: ParkingConfig,
) -> Dict[int, str]:
    """
    Classify every parking slot in *all_detections* at once.

    Instead of checking every object against every slot, each non-slot
    object is assigned to its **nearest** slot (by foot-point distance to
    slot center).  Then each slot is classified using only its own objects:

      - cone or disabled-person sign assigned → INVALID
      - P signboard assigned (center_x in slot x-range, base_y within
        P_SIGN_PROXIMITY_PX of slot top) → VALID
      - nothing assigned → UNKNOWN

    Returns a dict mapping the slot Detection's id() → status string.
    """
    slots = [d for d in all_detections if d.class_name == cfg.CLASS_PARKING_SLOT]
    others = [
        d for d in all_detections
        if d.class_name in (cfg.CLASS_CONE, cfg.CLASS_DISABLED_SIGN, cfg.CLASS_P_SIGN)
    ]

    if not slots:
        return {}

    # Assign each non-slot object to the nearest slot
    # slot_objects:  id(slot) → list of Detection
    slot_objects: Dict[int, List[Detection]] = {id(s): [] for s in slots}

    for obj in others:
        best_slot = min(slots, key=lambda s: _dist_to_slot(obj, s, cfg))
        slot_objects[id(best_slot)].append(obj)

    # Classify each slot based on its assigned objects only
    result: Dict[int, str] = {}
    for slot in slots:
        has_p_sign = False
        has_cone = False
        has_disabled = False

        for det in slot_objects[id(slot)]:
            if det.class_name in (cfg.CLASS_DISABLED_SIGN, cfg.CLASS_CONE):
                fx, fy = _foot_point(det)
                if slot.contains_point(fx, fy):
                    if det.class_name == cfg.CLASS_DISABLED_SIGN:
                        has_disabled = True
                    else:
                        has_cone = True

            elif det.class_name == cfg.CLASS_P_SIGN:
                sign_cx = det.center_x
                sign_base_y = det.y2
                in_x = slot.x1 <= sign_cx <= slot.x2
                in_y = (slot.y1 - cfg.P_SIGN_PROXIMITY_PX) <= sign_base_y <= slot.y2
                if in_x and in_y:
                    has_p_sign = True

        if has_disabled or has_cone:
            result[id(slot)] = "INVALID"
        elif has_p_sign:
            result[id(slot)] = "VALID"
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
    """Track whether a condition is stably true or false across frames."""

    def __init__(self, found_thresh: int = 3, lost_thresh: int = 5):
        self.found_thresh = found_thresh
        self.lost_thresh = lost_thresh
        self.consecutive_found = 0
        self.consecutive_lost = 0
        self.was_confirmed = False

    def update(self, detected: bool) -> str:
        if detected:
            self.consecutive_found += 1
            self.consecutive_lost = 0
            if self.consecutive_found >= self.found_thresh:
                self.was_confirmed = True
                return "CONFIRMED_FOUND"
            return "TRACKING"
        else:
            self.consecutive_lost += 1
            self.consecutive_found = 0
            if self.was_confirmed and self.consecutive_lost >= self.lost_thresh:
                return "CONFIRMED_LOST"
            if self.was_confirmed:
                return "UNCERTAIN"
            return "NOT_FOUND"

    def reset(self):
        self.consecutive_found = 0
        self.consecutive_lost = 0
        self.was_confirmed = False


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
      SCAN → COASTING → WAIT_STOP_ACK → WAIT_TURN_ACK → ALIGN
           → WAIT_ENTER_ACK → PARKED → WAIT_ROTATE_ACK → COMPLETE
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
        elif self.state == ParkingState.ALIGN:
            self._handle_align(detections)
        # WAIT_STOP_ACK, WAIT_TURN_ACK, WAIT_ENTER_ACK, WAIT_ROTATE_ACK
        #   → driven by on_ack() calls from the vision service when AnC responds

        return self.state

    def on_ack(self, ack_type: str):
        """
        Called by the vision service when AnC acknowledges a command.

        ack_type: "stop", "turn_complete", "enter_complete", "rotate_complete"
        """
        if ack_type == "stop" and self.state == ParkingState.WAIT_STOP_ACK:
            logger.info("ACK: stop — sending turn right 90°")
            self._emit(NavCommand(command="PAN_CAMERA_CENTER"))
            self._emit(NavCommand(command="TURN_RIGHT_90"))
            self._transition(ParkingState.WAIT_TURN_ACK)

        elif ack_type == "turn_complete" and self.state == ParkingState.WAIT_TURN_ACK:
            logger.info("ACK: turn complete — starting alignment")
            self.pi_controller.reset()
            self._align_attempts = 0
            self._last_align_time = time.time()
            self._transition(ParkingState.ALIGN)

        elif ack_type == "enter_complete" and self.state == ParkingState.WAIT_ENTER_ACK:
            logger.info("ACK: enter complete — PARKED")
            self._emit(NavCommand(command="STOP"))
            self._transition(ParkingState.PARKED)

        elif ack_type == "rotate_complete" and self.state == ParkingState.WAIT_ROTATE_ACK:
            logger.info("ACK: rotate complete — parking COMPLETE")
            self._emit(NavCommand(command="RESUME_LANE_TRACKING"))
            self._transition(ParkingState.COMPLETE)

    def trigger_exit(self):
        """Call this to initiate parking exit sequence."""
        if self.state == ParkingState.PARKED:
            logger.info("Triggering exit — rotate 180°")
            self._emit(NavCommand(command="ROTATE_180"))
            self._transition(ParkingState.WAIT_ROTATE_ACK)

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
            self._transition(ParkingState.WAIT_STOP_ACK)

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
            logger.info(f"Aligned! error={error_px:.1f}px — sending ENTER_SLOT")
            self._emit(NavCommand(command="STOP"))
            self.pi_controller.reset()
            self._emit(NavCommand(command="ENTER_SLOT"))
            self._transition(ParkingState.WAIT_ENTER_ACK)
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
            self._emit(NavCommand(command="ENTER_SLOT"))
            self._transition(ParkingState.WAIT_ENTER_ACK)

    # ----------------------------------------------------------
    # Utilities
    # ----------------------------------------------------------

    def _transition(self, new_state: ParkingState):
        logger.info(f"Parking: {self.state.name} → {new_state.name}")
        self.state = new_state

    def _emit(self, cmd: NavCommand):
        logger.info(f"  >> NAV: {cmd.to_dict()}")
        self.on_command(cmd)