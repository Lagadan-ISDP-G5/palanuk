#ifndef NSM_LINE_DETECTION_H
#define NSM_LINE_DETECTION_H

#include <opencv2/core.hpp>
#include "types.h"

namespace nsm {

void detect_line_sliding_window(const cv::Mat& thresh, const PipelineConfig& config, LineDetectionResult& out);
void detect_horizontal_line(const cv::Mat& thresh, int start_y, const PipelineConfig& config, LineDetectionResult& out);

}  // namespace nsm

#endif  // NSM_LINE_DETECTION_H
