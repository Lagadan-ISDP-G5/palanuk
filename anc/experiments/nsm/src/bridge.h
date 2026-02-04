#ifndef NSM_BRIDGE_H
#define NSM_BRIDGE_H

#include <cstdint>
#include <opencv2/core.hpp>
#include "types.h"
#include <optional>

namespace nsm {

struct BridgeResult {
    std::optional<float> heading_error;
    std::optional<float> abs_line_gradient;
    bool corner_detected = false;
    cv::Point2f corner_direction;
    cv::Point2f corner_point;

    void reset() {
        heading_error = std::nullopt;
        abs_line_gradient = std::nullopt;
        corner_detected = false;
        corner_direction = cv::Point2f();
        corner_point = cv::Point2f();
    }
};

// IPC message types (POD structs for zero-copy transfer)
struct HeadingErrorMsg {
    static constexpr const char* IOX2_TYPE_NAME = "HeadingErrorMsg";
    uint8_t valid;
    float value;
};

struct AbsLineGradientMsg {
    static constexpr const char* IOX2_TYPE_NAME = "AbsLineGradientMsg";
    uint8_t valid;
    float value;
};

struct CornerDetectedMsg {
    static constexpr const char* IOX2_TYPE_NAME = "CornerDetectedMsg";
    uint8_t detected;
};

struct CornerDirectionMsg {
    static constexpr const char* IOX2_TYPE_NAME = "CornerDirectionMsg";
    float x;
    float y;
};

struct CornerPointMsg {
    static constexpr const char* IOX2_TYPE_NAME = "CornerPointMsg";
    float x;
    float y;
};

// Initialize iceoryx2 publishers. Call once at startup.
bool init_publishers();

// Cleanup iceoryx2 publishers. Call at shutdown.
void shutdown_publishers();

void process(const FrameResult& frame_result, int frame_width, BridgeResult& out);
bool publish_control_vars(const BridgeResult& result);

}  // namespace nsm

#endif  // NSM_BRIDGE_H
