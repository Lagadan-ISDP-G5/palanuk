#ifndef NSM_CORNER_DETECTION_H
#define NSM_CORNER_DETECTION_H

#include <opencv2/core.hpp>
#include "types.h"

namespace nsm {

void detect_harris_corners(const cv::Mat& thresh, const PipelineConfig& config, std::vector<cv::Point2f>& out);
void detect_L_corner(const cv::Mat& thresh, const LineDetectionResult& center_line, const PipelineConfig& config,
                     LineDetectionResult& horiz_scratch, std::vector<cv::Point2f>& harris_scratch,
                     CornerDetectionResult& out);

}  // namespace nsm

#endif  // NSM_CORNER_DETECTION_H
