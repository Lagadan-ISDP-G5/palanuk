#ifndef NSM_LINE_DETECTION_H
#define NSM_LINE_DETECTION_H

#include <opencv2/core.hpp>
#include "types.h"
#include <optional>

namespace nsm {

void detect_line_sliding_window(const cv::Mat& thresh, const PipelineConfig& config, LineDetectionResult& out);
void detect_horizontal_line(const cv::Mat& thresh, int start_y, const PipelineConfig& config, LineDetectionResult& out);

// Returns normalized horizontal offset of the line's lower points.
// 0 = center, -1 = far left, +1 = far right.
// Uses average of up to 3 points closest to the bottom of the frame.
std::optional<float> calculate_heading_error(const LineDetectionResult& result, int frame_width);

// Returns the absolute value of the gradient (dx/dy) of the fitted line.
// A value of 0 means perfectly vertical; larger values indicate more tilt.
// Returns 0 if the line is not valid.
std::optional<float> calculate_abs_line_gradient(const LineDetectionResult& result);

}  // namespace nsm

#endif  // NSM_LINE_DETECTION_H
