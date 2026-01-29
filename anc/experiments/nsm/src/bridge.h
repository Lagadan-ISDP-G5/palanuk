#ifndef NSM_BRIDGE_H
#define NSM_BRIDGE_H

#include <opencv2/core.hpp>
#include "types.h"
#include <optional>

namespace nsm {

struct BridgeResult {
    std::optional<float> heading_error;
    std::optional<float> line_gradient_abs;
    bool corner_detected = false;
    cv::Point2f corner_direction;
    cv::Point2f corner_point;

    void reset() {
        heading_error = std::nullopt;
        line_gradient_abs = std::nullopt;
        corner_detected = false;
        corner_direction = cv::Point2f();
        corner_point = cv::Point2f();
    }
};

void process(const FrameResult& frame_result, int frame_width, BridgeResult& out);

// TODO: implement
void publish(const BridgeResult& result);

}  // namespace nsm

#endif  // NSM_BRIDGE_H
