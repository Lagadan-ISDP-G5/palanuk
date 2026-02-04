import cv2
import time
import json
import os
from datetime import datetime
from ultralytics import YOLO

# =========================
# CONFIG
# =========================
MODEL_PATH = "models/best.onnx"
IMG_SIZE = 640
CONF_THRES = 0.4
DEVICE = "cpu"

# =========================
# LOG SETUP (AUTO PER RUN)
# =========================
LOG_DIR = "logs"
os.makedirs(LOG_DIR, exist_ok=True)

run_id = datetime.now().strftime("%Y-%m-%d_%H-%M-%S")
LOG_PATH = os.path.join(LOG_DIR, f"run_{run_id}.jsonl")

# =========================
# LOAD MODEL
# =========================
model = YOLO(MODEL_PATH)

# =========================
# CAMERA SETUP
# =========================
cap = cv2.VideoCapture(0)
cap.set(cv2.CAP_PROP_FRAME_WIDTH, 640)
cap.set(cv2.CAP_PROP_FRAME_HEIGHT, 480)

frame_id = 0

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
        "device": DEVICE
    }
    log_file.write(json.dumps({"meta": meta}) + "\n")

    # =========================
    # MAIN LOOP
    # =========================
    while True:
        ret, frame = cap.read()
        if not ret:
            break

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
            "timestamp": timestamp,
            "detections": []
        }

        # ---- Bounding boxes ----
        if result.boxes is not None:
            for box in result.boxes:
                frame_log["detections"].append({
                    "type": "box",
                    "class_id": int(box.cls[0]),
                    "confidence": float(box.conf[0]),
                    "xyxy": box.xyxy[0].tolist()
                })

        # ---- Segmentation masks ----
        if result.masks is not None:
            for mask, cls, conf in zip(
                result.masks.xy,
                result.boxes.cls,
                result.boxes.conf
            ):
                frame_log["detections"].append({
                    "type": "mask",
                    "class_id": int(cls),
                    "confidence": float(conf),
                    "polygon": mask.tolist()
                })

        # ---- Write log ----
        log_file.write(json.dumps(frame_log) + "\n")
        log_file.flush()

        # ---- Visualization ----
        annotated = result.plot()
        cv2.imshow("YOLOv11-Seg ONNX", annotated)

        frame_id += 1

        # ESC to quit
        if cv2.waitKey(1) & 0xFF == 27:
            break

# =========================
# CLEANUP
# =========================
cap.release()
cv2.destroyAllWindows()
