#ifndef NSM_PREPROCESSING_H
#define NSM_PREPROCESSING_H

#include <opencv2/core.hpp>
#include "types.h"

namespace nsm {

cv::Mat threshold_white_line(const cv::Mat& img, const PipelineConfig& config);
void warp_birdseye(const cv::Mat& frame, const PipelineConfig& config, cv::Mat& out);

}  // namespace nsm

#endif  // NSM_PREPROCESSING_H
