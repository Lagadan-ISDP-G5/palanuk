#include "pipeline.h"
#include "preprocessing.h"
#include "line_detection.h"
#include "corner_detection.h"
#include <opencv2/core.hpp>

namespace nsm {

Pipeline::Pipeline(const PipelineConfig& config) : config_(config) {}

FrameResult Pipeline::process(const cv::Mat& frame) {
    FrameResult result;

    auto start = cv::getTickCount();

    // Stage 1: Preprocessing
    result.thresholded = threshold_white_line(frame, config_);

    // Stage 2: Line detection
    result.center_line = detect_line_sliding_window(result.thresholded, config_);

    // Stage 3: Corner detection
    result.corner = detect_L_corner(result.thresholded, result.center_line, config_);

    auto end = cv::getTickCount();
    result.processing_time_ms = (end - start) / cv::getTickFrequency() * 1000.0;

    return result;
}

void Pipeline::setConfig(const PipelineConfig& config) {
    config_ = config;
}

const PipelineConfig& Pipeline::getConfig() const {
    return config_;
}

}  // namespace nsm
