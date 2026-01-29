#include "bridge.h"
#include "line_detection.h"

namespace nsm {

void process(const FrameResult& frame_result, int frame_width, BridgeResult& out) {
    out.reset();

    out.heading_error = calculate_heading_error(frame_result.center_line, frame_width);
    out.abs_line_gradient = calculate_abs_line_gradient(frame_result.center_line);
    out.corner_detected = frame_result.corner.detected;
    out.corner_direction = frame_result.corner.horizontal_direction;
    out.corner_point = frame_result.corner.corner_point;
}

void publish(const BridgeResult& result) {
    // TODO: implement publishing logic
}

}  // namespace nsm
