#ifndef NSM_VISUALIZATION_H
#define NSM_VISUALIZATION_H

#include <opencv2/core.hpp>
#include "types.h"

namespace nsm {

cv::Mat visualize_result(const cv::Mat& original, const FrameResult& result);
cv::Mat visualize_side_by_side(const cv::Mat& original, const FrameResult& result);

}  // namespace nsm

#endif  // NSM_VISUALIZATION_H
