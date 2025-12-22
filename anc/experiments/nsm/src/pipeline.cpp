#include "pipeline.h"
#include "preprocessing.h"
#include "line_detection.h"
#include "corner_detection.h"
#include <opencv2/core.hpp>

namespace nsm {

Pipeline::Pipeline(const PipelineConfig& config) : config_(config) {}

const FrameResult& Pipeline::process(const cv::Mat& frame) {
    result_.reset();

    auto start = cv::getTickCount();

    // Stage 1: Preprocessing
    result_.thresholded = threshold_white_line(frame, config_);

    // Stage 2: Line detection
    detect_line_sliding_window(result_.thresholded, config_, result_.center_line);

    // Stage 3: Corner detection
    detect_L_corner(result_.thresholded, result_.center_line, config_,
                    horiz_line_scratch_, harris_corners_scratch_, result_.corner);

    auto end = cv::getTickCount();
    result_.processing_time_ms = (end - start) / cv::getTickFrequency() * 1000.0;

    return result_;
}

void Pipeline::setConfig(const PipelineConfig& config) {
    config_ = config;
}

const PipelineConfig& Pipeline::getConfig() const {
    return config_;
}

}  // namespace nsm
