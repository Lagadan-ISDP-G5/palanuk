#ifndef NSM_CORNER_DETECTION_H
#define NSM_CORNER_DETECTION_H

#include <opencv2/core.hpp>
#include "types.h"

namespace nsm {

std::vector<cv::Point2f> detect_harris_corners(const cv::Mat& thresh, const PipelineConfig& config);
CornerDetectionResult detect_L_corner(const cv::Mat& thresh, const LineDetectionResult& center_line, const PipelineConfig& config);

}  // namespace nsm

#endif  // NSM_CORNER_DETECTION_H
