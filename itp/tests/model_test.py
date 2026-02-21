import cv2
import time
import json
import os
from datetime import datetime
from ultralytics import YOLO
import torch
import numpy as np

# =========================
# CONFIG
# =========================
MODEL_PATH = "tests/best.onnx"
IMG_SIZE = 640
CONF_THRES = 0.4
DEVICE = "0"  # Changed from "gpu" to "0" (first GPU) or "cuda:0"
TASK = "segment"  # Keep segment task

# Frame capture settings
SAVE_FRAMES = False  # Enable/disable frame saving
CAPTURE_INTERVAL = 0  # seconds between frame captures (0 for every frame)
MAX_FRAMES = 100  # max frames to save (0 for unlimited)
SKIP_FRAMES = 10  # Process every Nth frame (1 = process all frames)

# Detection settings
USE_ONLY_BOXES = True  # Set to True to ignore segmentation masks, False to use both

# =========================
# LOG SETUP (AUTO PER RUN)
# =========================
LOG_DIR = "tests/logs"
os.makedirs(LOG_DIR, exist_ok=True)

run_id = datetime.now().strftime("%Y-%m-%d_%H-%M-%S")
LOG_PATH = os.path.join(LOG_DIR, f"run_{run_id}.jsonl")

# =========================
# FRAME CAPTURE SETUP
# =========================
saved_frame_count = 0
last_capture_time = 0  # Moved outside if block

if SAVE_FRAMES:
    FRAMES_DIR = os.path.join("tests/captured_frames", f"run_{run_id}")
    os.makedirs(FRAMES_DIR, exist_ok=True)

# =========================
# LOAD MODEL
# =========================
model = YOLO(MODEL_PATH, task=TASK)

# Get class names from model
class_names = model.names  # Dictionary: {0: 'class_name', 1: 'another_class', ...}
print(f"✓ Model loaded with {len(class_names)} classes: {class_names}")

# Generate distinct colors for each class
def generate_colors(num_classes):
    """Generate visually distinct colors for each class"""
    np.random.seed(42)  # For consistent colors across runs
    colors = {}
    for i in range(num_classes):
        # Generate random BGR colors
        colors[i] = tuple(map(int, np.random.randint(0, 255, 3)))
    return colors

CLASS_COLORS = generate_colors(len(class_names))
print(f"✓ Generated colors for {len(CLASS_COLORS)} classes")

# =========================
# CAMERA SETUP (RTSP)
# =========================
STREAM_URL = "rtsp://192.168.93.163:8554/camera"
MAX_RETRIES = 5
retry_count = 0

print(f"Connecting to RTSP stream: {STREAM_URL}")
cap = None
while retry_count < MAX_RETRIES:
    print(f"Attempt {retry_count + 1}/{MAX_RETRIES}...")
    cap = cv2.VideoCapture(STREAM_URL)
    if cap.isOpened():
        print("✓ Connected successfully!")
        break
    else:
        print(f"✗ Failed to connect, retrying in 2 seconds...")
        time.sleep(2)
        retry_count += 1

if cap is None or not cap.isOpened():
    print("Error: Could not connect to stream after multiple attempts")
    exit(1)

cap.set(cv2.CAP_PROP_FRAME_WIDTH, 640)
cap.set(cv2.CAP_PROP_FRAME_HEIGHT, 480)

frame_id = 0
processed_frame_count = 0

# =========================
# OPEN LOG FILE
# =========================
with open(LOG_PATH, "w") as log_file:
    # ---- Write run metadata first ----
    meta = {
        "run_id": run_id,
        "start_time": time.time(),
        "model": MODEL_PATH,
        "imgsz": IMG_SIZE,
        "conf_threshold": CONF_THRES,
        "device": DEVICE,
        "frame_capture_enabled": SAVE_FRAMES,
        "stream_url": STREAM_URL,
        "skip_frames": SKIP_FRAMES,
        "use_only_boxes": USE_ONLY_BOXES
    }
    log_file.write(json.dumps({"meta": meta}) + "\n")

    # =========================
    # MAIN LOOP
    # =========================
    while True:
        ret, frame = cap.read()
        if not ret:
            print("Failed to read frame, attempting reconnect...")
            cap.release()
            time.sleep(1)
            cap = cv2.VideoCapture(STREAM_URL)
            continue

        # Skip frames
        if frame_id % SKIP_FRAMES != 0:
            frame_id += 1
            continue

        # ---- Resize frame to 640 ----
        frame_resized = cv2.resize(frame, (IMG_SIZE, IMG_SIZE))

        timestamp = time.time()

        # ---- Inference ----
        result = model.predict(
            source=frame_resized,
            imgsz=IMG_SIZE,
            conf=CONF_THRES,
            device=DEVICE,
            verbose=False
        )[0]

        frame_log = {
            "frame_id": frame_id,
            "processed_frame_id": processed_frame_count,
            "timestamp": timestamp,
            "detections": []
        }

        # ---- Bounding boxes (ALWAYS logged) ----
        if result.boxes is not None:
            for box in result.boxes:
                class_id = int(box.cls[0])
                frame_log["detections"].append({
                    "type": "box",
                    "class_id": class_id,
                    "class_name": class_names[class_id],  # Add class name
                    "confidence": float(box.conf[0]),
                    "xyxy": box.xyxy[0].tolist()
                })

        # ---- Segmentation masks (ONLY if USE_ONLY_BOXES is False) ----
        if not USE_ONLY_BOXES and result.masks is not None:
            for mask, cls, conf in zip(
                result.masks.xy,
                result.boxes.cls,
                result.boxes.conf
            ):
                class_id = int(cls)
                frame_log["detections"].append({
                    "type": "mask",
                    "class_id": class_id,
                    "class_name": class_names[class_id],  # Add class name
                    "confidence": float(conf),
                    "polygon": mask.tolist()
                })

        # ---- Write log ----
        log_file.write(json.dumps(frame_log) + "\n")
        log_file.flush()

        # ---- Save Frame ----
        if SAVE_FRAMES and (timestamp - last_capture_time >= CAPTURE_INTERVAL):
            filename = f"frame_{frame_id:05d}_{int(timestamp*1000)}.jpg"
            filepath = os.path.join(FRAMES_DIR, filename)
            cv2.imwrite(filepath, frame)
            
            saved_frame_count += 1
            last_capture_time = timestamp
            
            # Check max frames limit
            if MAX_FRAMES > 0 and saved_frame_count >= MAX_FRAMES:
                print(f"\nReached max frames ({MAX_FRAMES}). Stopping...")
                break

        # ---- Visualization ----
        # If USE_ONLY_BOXES, visualize only boxes
        if USE_ONLY_BOXES:
            annotated = frame_resized.copy()
            if result.boxes is not None:
                for box in result.boxes:
                    x1, y1, x2, y2 = map(int, box.xyxy[0])
                    cls_id = int(box.cls[0])
                    conf = float(box.conf[0])
                    
                    # Get color for this class
                    color = CLASS_COLORS[cls_id]
                    
                    # Draw bounding box
                    cv2.rectangle(annotated, (x1, y1), (x2, y2), color, 2)
                    
                    # Draw label with class name
                    class_name = class_names[cls_id]
                    label = f"{class_name}: {conf:.2f}"
                    
                    # Draw label background
                    (label_w, label_h), _ = cv2.getTextSize(
                        label, cv2.FONT_HERSHEY_SIMPLEX, 0.5, 2
                    )
                    cv2.rectangle(
                        annotated, 
                        (x1, y1 - label_h - 10), 
                        (x1 + label_w, y1), 
                        color, 
                        -1
                    )
                    
                    # Draw label text
                    cv2.putText(
                        annotated, 
                        label, 
                        (x1, y1 - 5),
                        cv2.FONT_HERSHEY_SIMPLEX, 
                        0.5, 
                        (255, 255, 255),  # White text
                        2
                    )
        else:
            # Use default plot (boxes + masks)
            annotated = result.plot()
        
        # Add frame info to display
        info_text = f"Processed: {processed_frame_count} | Mode: {'Boxes Only' if USE_ONLY_BOXES else 'Boxes + Masks'}"
        cv2.putText(annotated, info_text, (10, 30),
                   cv2.FONT_HERSHEY_SIMPLEX, 0.7, (0, 255, 0), 2)
        
        cv2.imshow("YOLOv11-Seg ONNX", annotated)

        frame_id += 1
        processed_frame_count += 1

        # ESC to quit
        if cv2.waitKey(1) & 0xFF == 27:
            break

# =========================
# CLEANUP
# =========================
cap.release()
cv2.destroyAllWindows()

if SAVE_FRAMES:
    print(f"\nTotal frames saved: {saved_frame_count}")
    print(f"Total frames processed: {processed_frame_count}")
    print(f"Location: {FRAMES_DIR}")