"""
Offline Test Script
====================
Tests the YOLO model + Zenoh bridge + parking state machine
on pre-recorded footage from tests/parking_footage/.

Controls:
  SPACE  — pause / resume
  N      — next video
  R      — restart current video
  S      — save current annotated frame as PNG
  A      — advance state manually (LANE_FOLLOWING → APPROACH_PARKING → PARKING)
  ESC    — quit
"""

import cv2
import os
import sys
import time
import json
import glob
import struct
import numpy as np
from datetime import datetime
from collections import deque
from ultralytics import YOLO

# ── make sure imports from the tests/ folder work ──
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from parking_service import (
    ParkingStateMachine,
    ParkingConfig,
    ParkingState,
    NavCommand,
    NAV_CMD_RECIPES,
    NAV_CMD_TOPICS,
    parse_detections,
    Detection,
    DebounceTracker,
    classify_slot,
    classify_all_slots,
)

# Import the Zenoh bridge & helpers from overall_algorithm
from overall_algorithm import (
    VisionConfig,
    ZenohBridge,
    OverallState,
    generate_colors,
)


# ============================================================
# Configuration
# ============================================================

FOOTAGE_DIR  = os.path.join(os.path.dirname(__file__), "parking_footage")
ONNX_PATH    = os.path.join(os.path.dirname(__file__), "best.onnx")
ENGINE_PATH  = os.path.join(os.path.dirname(__file__), "best.engine")
IMG_SIZE     = 640
CONF_THRES   = 0.4
DEVICE       = "0"        # GPU; change to "cpu" if no CUDA
TASK         = "segment"
USE_FP16     = True       # half-precision inference

# Auto-select TensorRT engine if available, else fall back to ONNX
if os.path.exists(ENGINE_PATH):
    MODEL_PATH = ENGINE_PATH
    print(f"[INFO] Using TensorRT engine: {ENGINE_PATH}")
else:
    MODEL_PATH = ONNX_PATH
    print(f"[INFO] TensorRT engine not found, using ONNX: {ONNX_PATH}")

WINDOW_NAME  = "Offline Test — Model + Zenoh"
SAVE_DIR     = os.path.join(os.path.dirname(__file__), "captured_frames",
                            f"test_{datetime.now().strftime('%Y-%m-%d_%H-%M-%S')}")
LOG_DIR      = os.path.join(os.path.dirname(__file__), "logs")

# Video recording — set to True to save annotated output videos
SAVE_VIDEO: bool = True
VIDEO_OUT_DIR = os.path.join(os.path.dirname(__file__), "output_videos",
                             f"run_{datetime.now().strftime('%Y-%m-%d_%H-%M-%S')}")

# Frame skipping — only run inference every N frames (1 = every frame)
PROCESS_EVERY_N_FRAMES: int = 2

# Valid parking area threshold — fraction of frame area.
# When any VALID parking slot exceeds this, auto-trigger APPROACH_PARKING.
VALID_PARKING_AREA_THRESHOLD: float = 0.1  # 10% of frame

# How many Zenoh messages to show in the overlay log
ZENOH_LOG_MAX = 8


# ============================================================
# Intercepting ZenohBridge to capture publishes for display
# ============================================================

class TestZenohBridge(ZenohBridge):
    """
    Subclass that captures every publish() / publish_bytes() call
    into a visible log so we can render it on the video overlay.
    """

    def __init__(self):
        super().__init__()
        self.publish_log: deque = deque(maxlen=50)

    def publish(self, topic: str, data):
        # Call parent (which does real Zenoh or console fallback)
        super().publish(topic, data)
        # Also capture for overlay display
        ts = datetime.now().strftime("%H:%M:%S.%f")[:-3]
        if isinstance(data, dict):
            short = json.dumps(data, separators=(",", ":"))
            if len(short) > 120:
                short = short[:117] + "..."
        else:
            short = str(data)[:120]
        self.publish_log.append(f"[{ts}] {topic}: {short}")

    def publish_bytes(self, topic: str, data: bytes):
        # Call parent
        super().publish_bytes(topic, data)
        # Decode value for human-readable log
        ts = datetime.now().strftime("%H:%M:%S.%f")[:-3]
        if len(data) == 1:
            val = struct.unpack("B", data)[0]
            self.publish_log.append(f"[{ts}] {topic}: {val}")
        elif len(data) == 8:
            val = struct.unpack("d", data)[0]
            self.publish_log.append(f"[{ts}] {topic}: {val:.3f}")
        else:
            self.publish_log.append(f"[{ts}] {topic}: {data.hex()}")


# ============================================================
# Color palette
# ============================================================

def make_colors(n: int):
    np.random.seed(42)
    return {i: tuple(map(int, np.random.randint(0, 255, 3))) for i in range(n)}


# ============================================================
# Draw helpers
# ============================================================

def draw_detections(frame, detections, class_colors, class_names, cfg: ParkingConfig):
    """Draw bounding boxes, labels, and slot validity tags."""
    # Classify all slots in one batch (nearest-slot assignment)
    slot_status_map = classify_all_slots(detections, cfg)

    for d in detections:
        color = class_colors.get(d.class_id, (0, 255, 0))
        x1, y1, x2, y2 = int(d.x1), int(d.y1), int(d.x2), int(d.y2)
        cv2.rectangle(frame, (x1, y1), (x2, y2), color, 2)

        label = f"{d.class_name} {d.confidence:.2f}"

        # If this is a parking slot, tag it VALID / INVALID / UNKNOWN
        if d.class_name == cfg.CLASS_PARKING_SLOT:
            slot_status = slot_status_map.get(id(d), "UNKNOWN")
            area_pct = d.area / (IMG_SIZE ** 2) * 100
            status_color = {
                "VALID":   (0, 255, 0),
                "INVALID": (0, 0, 255),
                "UNKNOWN": (0, 200, 255),
            }.get(slot_status, (200, 200, 200))
            label += f" [{slot_status} {area_pct:.1f}%]"
            # Highlight the slot border with status color
            cv2.rectangle(frame, (x1, y1), (x2, y2), status_color, 3)
            # Draw slot center marker
            scx, scy = int(d.center_x), int(d.center_y)
            cv2.drawMarker(frame, (scx, scy), status_color,
                           cv2.MARKER_TILTED_CROSS, 16, 2)

        # Draw foot-point (center_x, y2) for cones, disabled signs, P signs
        if d.class_name in (cfg.CLASS_CONE, cfg.CLASS_DISABLED_SIGN, cfg.CLASS_P_SIGN):
            foot_x, foot_y = int(d.center_x), int(d.y2)
            cv2.circle(frame, (foot_x, foot_y), 6, (0, 255, 255), -1)   # yellow dot
            cv2.drawMarker(frame, (foot_x, foot_y), (0, 255, 255),
                           cv2.MARKER_CROSS, 14, 2)                      # crosshair

        (tw, th), _ = cv2.getTextSize(label, cv2.FONT_HERSHEY_SIMPLEX, 0.5, 2)
        cv2.rectangle(frame, (x1, y1 - th - 10), (x1 + tw, y1), color, -1)
        cv2.putText(frame, label, (x1, y1 - 5),
                    cv2.FONT_HERSHEY_SIMPLEX, 0.5, (255, 255, 255), 2)


def draw_overlay(frame, video_name, frame_no, total_frames, fps,
                 overall_state, parking_state, det_count, paused,
                 zenoh_ok, zenoh_log, infer_ms):
    """HUD overlay with state info and Zenoh publish log."""
    h, w = frame.shape[:2]

    # ── Top bar ──
    overlay = frame.copy()
    cv2.rectangle(overlay, (0, 0), (w, 110), (0, 0, 0), -1)
    cv2.addWeighted(overlay, 0.6, frame, 0.4, 0, frame)

    y = 18
    cv2.putText(frame, f"Video: {video_name}", (10, y),
                cv2.FONT_HERSHEY_SIMPLEX, 0.5, (255, 255, 255), 1)
    y += 20
    cv2.putText(frame, f"Frame: {frame_no}/{total_frames}  |  FPS: {fps:.1f}  |  "
                f"Infer: {infer_ms:.0f}ms  |  Objects: {det_count}",
                (10, y), cv2.FONT_HERSHEY_SIMPLEX, 0.45, (200, 200, 200), 1)
    y += 20
    state_txt = f"State: {overall_state}"
    if parking_state:
        state_txt += f"  |  Parking: {parking_state}"
    cv2.putText(frame, state_txt, (10, y),
                cv2.FONT_HERSHEY_SIMPLEX, 0.5, (0, 200, 255), 1)
    y += 20
    zenoh_txt = "Zenoh: CONNECTED" if zenoh_ok else "Zenoh: CONSOLE MODE"
    zenoh_clr = (0, 255, 0) if zenoh_ok else (0, 150, 255)
    cv2.putText(frame, zenoh_txt, (10, y),
                cv2.FONT_HERSHEY_SIMPLEX, 0.45, zenoh_clr, 1)

    if paused:
        cv2.putText(frame, "PAUSED", (w - 120, 30),
                    cv2.FONT_HERSHEY_SIMPLEX, 0.8, (0, 0, 255), 2)

    # ── Zenoh publish log ──
    log_entries = list(zenoh_log)[-ZENOH_LOG_MAX:]
    if log_entries:
        log_x = 10
        log_y = 125
        cv2.putText(frame, "Zenoh Publish Log:", (log_x, log_y - 5),
                    cv2.FONT_HERSHEY_SIMPLEX, 0.4, (0, 255, 255), 1)
        for entry in log_entries:
            display = entry if len(entry) < 90 else entry[:87] + "..."
            cv2.putText(frame, display, (log_x, log_y + 12),
                        cv2.FONT_HERSHEY_SIMPLEX, 0.3, (180, 255, 180), 1)
            log_y += 14

    # ── Key guide at bottom ──
    guide = "SPACE=pause  N=next  R=restart  S=snapshot  A=advance state  ESC=quit"
    cv2.putText(frame, guide, (10, h - 10),
                cv2.FONT_HERSHEY_SIMPLEX, 0.4, (180, 180, 180), 1)


# ============================================================
# Zenoh test helper
# ============================================================

def test_zenoh_bridge():
    """Open a Zenoh session, declare publishers, send a test message, close."""
    print("\n" + "=" * 50)
    print("ZENOH BRIDGE TEST")
    print("=" * 50)
    bridge = ZenohBridge()
    bridge.open()

    for t in NAV_CMD_TOPICS:
        bridge.declare_publisher(t)

    # Send a test ping (STOP=0 → harmless resume)
    test_cmd = NavCommand(command="RESUME_LANE_TRACKING")
    test_cmd.publish_all(bridge)

    print(f"  -> Zenoh available: {bridge._zenoh_available}")
    print(f"  -> Publishers: {list(bridge.publishers.keys())}")
    bridge.close()
    return bridge._zenoh_available


# ============================================================
# Main
# ============================================================

def main():
    # ── discover videos ──
    videos = sorted(glob.glob(os.path.join(FOOTAGE_DIR, "*.mp4")))
    if not videos:
        print(f"No .mp4 files found in {FOOTAGE_DIR}")
        return

    print(f"Found {len(videos)} video(s):")
    for i, v in enumerate(videos):
        print(f"  [{i}] {os.path.basename(v)}")

    # ── test Zenoh up-front ──
    zenoh_ok = test_zenoh_bridge()

    # ── load model ──
    print(f"\nLoading model: {MODEL_PATH}")
    model = YOLO(MODEL_PATH, task=TASK)
    class_names = model.names
    class_colors = make_colors(len(class_names))
    print(f"Loaded — {len(class_names)} classes: {class_names}")
    print(f"FP16={USE_FP16}, TensorRT={'yes' if MODEL_PATH.endswith('.engine') else 'no'}")
    print(f"Processing every {PROCESS_EVERY_N_FRAMES} frame(s)")
    print(f"Valid parking area threshold: {VALID_PARKING_AREA_THRESHOLD*100:.1f}%")

    # ── parking helpers ──
    pcfg = ParkingConfig()
    vcfg = VisionConfig()

    # ── overall state (simple simulation) ──
    overall_state = OverallState.LANE_FOLLOWING
    bumper_tracker = DebounceTracker(found_thresh=vcfg.BUMPER_DEBOUNCE_FRAMES, lost_thresh=3)
    valid_slot_tracker = DebounceTracker(found_thresh=3, lost_thresh=5)
    parking_sm = None

    # ── Zenoh bridge for live publishing during playback ──
    zenoh = TestZenohBridge()
    zenoh.open()
    for topic in NAV_CMD_TOPICS:
        zenoh.declare_publisher(topic)
    if vcfg.DEBUG_PUBLISH:
        zenoh.declare_publisher(vcfg.ZENOH_TOPIC_STATE)
        zenoh.declare_publisher(vcfg.ZENOH_TOPIC_DETECTIONS)

    # Initialise all bstn topics to safe state
    NavCommand(command="INIT_SAFE_STATE").publish_all(zenoh)

    # ── Log file setup ──
    os.makedirs(LOG_DIR, exist_ok=True)
    run_ts = datetime.now().strftime("%Y-%m-%d_%H-%M-%S")
    log_path = os.path.join(LOG_DIR, f"test_{run_ts}.jsonl")
    log_file = open(log_path, "w")
    log_file.write(json.dumps({
        "meta": {
            "run_id": run_ts,
            "start_time": time.time(),
            "model": MODEL_PATH,
            "classes": {str(k): v for k, v in class_names.items()},
            "process_every_n": PROCESS_EVERY_N_FRAMES,
            "valid_parking_area_threshold": VALID_PARKING_AREA_THRESHOLD,
            "conf_threshold": CONF_THRES,
        },
    }) + "\n")
    print(f"Logging to: {log_path}")

    # ── iterate videos ──
    vid_idx = 0
    while vid_idx < len(videos):
        video_path = videos[vid_idx]
        video_name = os.path.basename(video_path)
        print(f"\n>>> Playing: {video_name}")

        cap = cv2.VideoCapture(video_path)
        if not cap.isOpened():
            print(f"  Cannot open {video_path}, skipping.")
            vid_idx += 1
            continue

        total_frames = int(cap.get(cv2.CAP_PROP_FRAME_COUNT))
        src_fps = cap.get(cv2.CAP_PROP_FPS) or 30.0
        frame_no = 0
        paused = False
        prev_time = time.time()
        infer_ms = 0.0

        # ── Video writer setup ──
        video_writer = None
        if SAVE_VIDEO:
            os.makedirs(VIDEO_OUT_DIR, exist_ok=True)
            out_name = os.path.splitext(video_name)[0] + "_annotated.mp4"
            out_path = os.path.join(VIDEO_OUT_DIR, out_name)
            fourcc = cv2.VideoWriter_fourcc(*"mp4v")
            video_writer = cv2.VideoWriter(out_path, fourcc, src_fps,
                                           (IMG_SIZE, IMG_SIZE))
            print(f"  Recording to: {out_path}")

        # Reset state for each video
        overall_state = OverallState.LANE_FOLLOWING
        bumper_tracker.reset()
        valid_slot_tracker.reset()
        parking_sm = None
        parking_state_name = None
        parked_triggered = False

        # Publish initial state (debug only)
        if vcfg.DEBUG_PUBLISH:
            zenoh.publish(vcfg.ZENOH_TOPIC_STATE, {
                "state": overall_state.name,
                "video": video_name,
                "timestamp": time.time(),
            })

        while True:
            if not paused:
                ret, frame = cap.read()
                if not ret:
                    print(f"  End of {video_name}")
                    break
                frame_no += 1

                # ── Frame skip — read but don't process every frame ──
                if frame_no % PROCESS_EVERY_N_FRAMES != 0:
                    continue

                # Resize for model
                frame_resized = cv2.resize(frame, (IMG_SIZE, IMG_SIZE))

                # ── YOLO inference ──
                t0 = time.time()
                result = model.predict(
                    source=frame_resized,
                    imgsz=IMG_SIZE,
                    conf=CONF_THRES,
                    device=DEVICE,
                    half=USE_FP16,
                    verbose=False,
                )[0]
                infer_ms = (time.time() - t0) * 1000

                detections = parse_detections(result, class_names, CONF_THRES)

                frame_area = IMG_SIZE ** 2

                # ── Publish detections via Zenoh (debug only) ──
                if vcfg.DEBUG_PUBLISH:
                    det_payload = {
                        "frame_id": frame_no,
                        "timestamp": time.time(),
                        "state": overall_state.name,
                        "video": video_name,
                        "count": len(detections),
                        "inference_ms": round(infer_ms, 1),
                        "objects": [
                            {
                                "class": d.class_name,
                                "conf": round(d.confidence, 3),
                                "cx": round(d.center_x, 1),
                                "cy": round(d.center_y, 1),
                                "area_pct": round(d.area / frame_area, 4),
                            }
                            for d in detections
                        ],
                    }
                    zenoh.publish(vcfg.ZENOH_TOPIC_DETECTIONS, det_payload)

                # ── Classify all parking slots this frame ──
                slot_status_map = classify_all_slots(detections, pcfg)
                slot_classifications = []
                for d in detections:
                    if d.class_name == pcfg.CLASS_PARKING_SLOT:
                        status_str = slot_status_map.get(id(d), "UNKNOWN")
                        slot_classifications.append({
                            "status": status_str,
                            "area_pct": round(d.area / frame_area, 4),
                            "cx": round(d.center_x, 1),
                            "cy": round(d.center_y, 1),
                        })

                # ── Log frame to JSONL ──
                log_entry = {
                    "frame_id": frame_no,
                    "timestamp": time.time(),
                    "video": video_name,
                    "state": overall_state.name,
                    "parking_state": parking_state_name,
                    "inference_ms": round(infer_ms, 1),
                    "detections": [
                        {
                            "class": d.class_name,
                            "conf": round(d.confidence, 3),
                            "xyxy": [round(d.x1, 1), round(d.y1, 1),
                                     round(d.x2, 1), round(d.y2, 1)],
                            "area_pct": round(d.area / frame_area, 4),
                        }
                        for d in detections
                    ],
                    "slots": slot_classifications,
                }
                log_file.write(json.dumps(log_entry) + "\n")
                if frame_no % 30 == 0:
                    log_file.flush()

                # ── Simple state machine simulation ──
                parking_state_name = None

                if overall_state == OverallState.LANE_FOLLOWING:
                    # Watch for bumpers — trigger when bumper y2 is in
                    # the bottom 5% of the frame (close to robot)
                    bumpers = [
                        d for d in detections
                        if d.class_name == vcfg.CLASS_BUMPER
                        and d.y2 >= IMG_SIZE * vcfg.BUMPER_Y2_THRESHOLD
                    ]
                    bmp_status = bumper_tracker.update(len(bumpers) > 0)
                    if bmp_status == "CONFIRMED_FOUND":  # fires only once
                        b = bumpers[0]
                        cmd = NavCommand(command="ACCELERATE_FOR_BUMP")
                        cmd.publish_all(zenoh)
                        print(f"    [!] Bumper detected (y2={b.y2:.0f}) → ACCELERATE_FOR_BUMP @ frame {frame_no}")

                    # Watch for valid parking slots exceeding area threshold
                    valid_slots = [
                        d for d in detections
                        if d.class_name == pcfg.CLASS_PARKING_SLOT
                        and slot_status_map.get(id(d), "UNKNOWN") == "VALID"
                        and d.area / frame_area >= VALID_PARKING_AREA_THRESHOLD
                    ]
                    slot_status = valid_slot_tracker.update(len(valid_slots) > 0)
                    if slot_status == "CONFIRMED_FOUND":  # fires only once
                        best = max(valid_slots, key=lambda s: s.area)
                        # PARKING_SLOTS_DETECTED is internal — no Zenoh publish
                        # In test mode, auto-transition (no real ACK from AnC)
                        overall_state = OverallState.APPROACH_PARKING
                        if vcfg.DEBUG_PUBLISH:
                            zenoh.publish(vcfg.ZENOH_TOPIC_STATE, {
                                "state": overall_state.name,
                                "timestamp": time.time(),
                                "trigger": "valid_parking_area",
                            })
                        print(f"    [!] Valid parking slot confirmed @ frame {frame_no} "
                              f"(area={best.area/frame_area:.2%}) → APPROACH_PARKING")

                elif overall_state == OverallState.APPROACH_PARKING:
                    # Pan camera right for parking scan
                    pan_cmd = NavCommand(command="PAN_CAMERA_RIGHT")
                    pan_cmd.publish_all(zenoh)
                    print(f"    [!] PAN_CAMERA_RIGHT sent @ frame {frame_no}")
                    # Camera is panned right so Pi can't lane-track — drive forward open-loop
                    fwd_cmd = NavCommand(command="DRIVE_FORWARD")
                    fwd_cmd.publish_all(zenoh)
                    print(f"    [!] DRIVE_FORWARD sent @ frame {frame_no} (camera panned, no lane tracking)")

                    # Initialize parking SM
                    parking_sm = ParkingStateMachine(
                        pcfg,
                        on_command=lambda cmd: cmd.publish_all(zenoh),
                    )
                    parked_triggered = False  # fire-once guard for PARKED
                    overall_state = OverallState.PARKING
                    if vcfg.DEBUG_PUBLISH:
                        zenoh.publish(vcfg.ZENOH_TOPIC_STATE, {
                            "state": overall_state.name,
                            "timestamp": time.time(),
                        })
                    print(f"    [!] Parking SM initialized @ frame {frame_no}")

                elif overall_state == OverallState.PARKING and parking_sm:
                    p_state = parking_sm.process_frame(result, class_names)
                    parking_state_name = parking_sm.state.name

                    if p_state == ParkingState.PARKED and not parked_triggered:
                        parked_triggered = True
                        print(f"    [!] PARKED @ frame {frame_no}")
                    elif p_state == ParkingState.COMPLETE:
                        overall_state = OverallState.FINISHED
                        if vcfg.DEBUG_PUBLISH:
                            zenoh.publish(vcfg.ZENOH_TOPIC_STATE, {
                                "state": overall_state.name,
                                "timestamp": time.time(),
                            })
                        print(f"    [!] Parking COMPLETE @ frame {frame_no}")
                    elif p_state == ParkingState.FAILED:
                        overall_state = OverallState.ERROR
                        if vcfg.DEBUG_PUBLISH:
                            zenoh.publish(vcfg.ZENOH_TOPIC_STATE, {
                                "state": overall_state.name,
                                "timestamp": time.time(),
                            })
                        print(f"    [!] Parking FAILED @ frame {frame_no}")

                # ── Draw annotations ──
                annotated = frame_resized.copy()
                draw_detections(annotated, detections, class_colors, class_names, pcfg)

                # Draw masks if available
                if result.masks is not None:
                    for i, mask in enumerate(result.masks.data):
                        mask_np = mask.cpu().numpy().astype(np.uint8)
                        mask_resized = cv2.resize(mask_np, (IMG_SIZE, IMG_SIZE))
                        cls_id = int(result.boxes.cls[i])
                        color = class_colors.get(cls_id, (0, 255, 0))
                        colored_mask = np.zeros_like(annotated)
                        colored_mask[mask_resized > 0] = color
                        annotated = cv2.addWeighted(annotated, 1.0, colored_mask, 0.35, 0)

                # FPS
                now = time.time()
                fps = 1.0 / max(now - prev_time, 1e-6)
                prev_time = now

                draw_overlay(annotated, video_name, frame_no, total_frames, fps,
                             overall_state.name, parking_state_name,
                             len(detections), paused, zenoh_ok,
                             zenoh.publish_log, infer_ms)

                # Write annotated frame to output video
                if video_writer is not None:
                    video_writer.write(annotated)

                cv2.imshow(WINDOW_NAME, annotated)

            # ── Key handling ──
            key = cv2.waitKey(1 if not paused else 50) & 0xFF
            if key == 27:  # ESC
                cap.release()
                log_file.close()
                zenoh.close()
                cv2.destroyAllWindows()
                print(f"\nLog saved: {log_path}")
                print("Exited by user.")
                return
            elif key == ord(' '):
                paused = not paused
            elif key == ord('n'):
                break  # next video
            elif key == ord('r'):
                cap.set(cv2.CAP_PROP_POS_FRAMES, 0)
                frame_no = 0
                overall_state = OverallState.LANE_FOLLOWING
                bumper_tracker.reset()
                valid_slot_tracker.reset()
                parking_sm = None
                parking_state_name = None
                parked_triggered = False
                zenoh.publish_log.clear()
                print(f"  Restarted {video_name}")
            elif key == ord('s'):
                os.makedirs(SAVE_DIR, exist_ok=True)
                snap_path = os.path.join(SAVE_DIR, f"frame_{frame_no:05d}.png")
                cv2.imwrite(snap_path, annotated)
                print(f"  Saved snapshot: {snap_path}")
            elif key == ord('a'):
                # Manual state advance for testing
                if overall_state == OverallState.LANE_FOLLOWING:
                    overall_state = OverallState.APPROACH_PARKING
                    if vcfg.DEBUG_PUBLISH:
                        zenoh.publish(vcfg.ZENOH_TOPIC_STATE, {
                            "state": overall_state.name,
                            "timestamp": time.time(),
                            "note": "manual_advance",
                        })
                    print(f"    [MANUAL] Advanced → APPROACH_PARKING")
                elif overall_state == OverallState.APPROACH_PARKING:
                    print(f"    [MANUAL] Will init parking SM on next frame")

        if video_writer is not None:
            video_writer.release()
            print(f"  Video saved: {out_path}")
        cap.release()
        vid_idx += 1

    # ── Cleanup ──
    log_file.close()
    zenoh.close()
    cv2.destroyAllWindows()
    print(f"\nLog saved: {log_path}")
    print("All videos processed. Done.")


if __name__ == "__main__":
    main()