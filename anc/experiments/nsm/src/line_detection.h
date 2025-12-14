#ifndef NSM_LINE_DETECTION_H
#define NSM_LINE_DETECTION_H

#include <opencv2/core.hpp>
#include "types.h"

namespace nsm {

LineDetectionResult detect_line_sliding_window(const cv::Mat& thresh, const PipelineConfig& config);
LineDetectionResult detect_horizontal_line(const cv::Mat& thresh, int start_y, const PipelineConfig& config);

}  // namespace nsm

#endif  // NSM_LINE_DETECTION_H
