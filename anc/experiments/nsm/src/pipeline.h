#ifndef NSM_PIPELINE_H
#define NSM_PIPELINE_H

#include <opencv2/core.hpp>
#include "types.h"

namespace nsm {

class Pipeline {
public:
    explicit Pipeline(const PipelineConfig& config = PipelineConfig{});

    FrameResult process(const cv::Mat& frame);

    void setConfig(const PipelineConfig& config);
    const PipelineConfig& getConfig() const;

private:
    PipelineConfig config_;
};

}  // namespace nsm

#endif  // NSM_PIPELINE_H
