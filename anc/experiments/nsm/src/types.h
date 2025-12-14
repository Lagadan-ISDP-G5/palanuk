#ifndef NSM_TYPES_H
#define NSM_TYPES_H

#include <opencv2/core.hpp>
#include <vector>

namespace nsm {

struct LineDetectionResult {
    std::vector<cv::Point2f> points;
    cv::Vec4f fitted_line;  // (vx, vy, x0, y0)
    bool valid = false;
};

struct CornerDetectionResult {
    cv::Point2f corner_point;
    cv::Point2f horizontal_direction;  // unit vector along horizontal line
    bool detected = false;
};

struct FrameResult {
    cv::Mat thresholded;
    LineDetectionResult center_line;
    CornerDetectionResult corner;
    double processing_time_ms = 0.0;
};

struct PipelineConfig {
    // ROI settings
    float roi_ignore_top_percent = 0.53f;

    // Thresholding
    int brightness_threshold = 200;
    int blur_kernel_size = 5;

    // Contour filtering
    float min_contour_length = 50.0f;
    float min_aspect_ratio = 2.5f;

    // Sliding window (vertical line)
    int num_windows = 10;
    int window_width = 100;
    int min_pixel_threshold = 50;

    // Horizontal line detection
    int horiz_num_windows = 10;
    int horiz_window_height = 40;

    // Corner detection
    int harris_block_size = 9;
    int harris_ksize = 3;
    double harris_k = 0.04;
    double harris_threshold = 150.0;
    float corner_max_distance = 100.0f;
    int max_corners = 10;
};

}  // namespace nsm

#endif  // NSM_TYPES_H
