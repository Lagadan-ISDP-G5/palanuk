#ifndef NSM_PIPELINE_H
#define NSM_PIPELINE_H

#include <opencv2/core.hpp>
#include "types.h"

namespace nsm {

class Pipeline {
public:
    explicit Pipeline(const PipelineConfig& config = PipelineConfig{});

    const FrameResult& process(const cv::Mat& frame);

    void setConfig(const PipelineConfig& config);
    const PipelineConfig& getConfig() const;

private:
    PipelineConfig config_;

    // reused each frame to avoid allocations
    FrameResult result_;
    LineDetectionResult horiz_line_scratch_;
    std::vector<cv::Point2f> harris_corners_scratch_;
};

}  // namespace nsm

#endif  // NSM_PIPELINE_H
