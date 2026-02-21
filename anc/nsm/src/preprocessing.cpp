#include "preprocessing.h"
#include <opencv2/imgproc.hpp>

namespace nsm {

void mask_out_yellow(const cv::Mat& img, const PipelineConfig& config, cv::Mat& out) {
    cv::Mat hsv, yellow_mask;
    cv::cvtColor(img, hsv, cv::COLOR_BGR2HSV);
    cv::inRange(hsv,
        cv::Scalar(config.yellow_h_low, config.yellow_s_low, config.yellow_v_low),
        cv::Scalar(config.yellow_h_high, 255, 255),
        yellow_mask);
    img.copyTo(out);
    out.setTo(cv::Scalar(0, 0, 0), yellow_mask);
}

cv::Mat threshold_white_line(const cv::Mat& img, const PipelineConfig& config) {
    cv::Mat gray, blurred, thresh_raw;
    cv::cvtColor(img, gray, cv::COLOR_BGR2GRAY);
    cv::GaussianBlur(gray, blurred, cv::Size(config.blur_kernel_size, config.blur_kernel_size), 0);
    cv::threshold(blurred, thresh_raw, config.brightness_threshold, 255, cv::THRESH_BINARY);

    // cv::adaptiveThreshold(blurred, thresh_raw, 255, cv::ADAPTIVE_THRESH_GAUSSIAN_C, cv::THRESH_BINARY, 15, 0);

    // Mask out the top portion of the frame
    int roi_top = static_cast<int>(img.rows * config.roi_ignore_top_percent);
    thresh_raw(cv::Rect(0, 0, img.cols, roi_top)) = 0;

    // Find contours and filter by shape
    std::vector<std::vector<cv::Point>> contours;
    cv::findContours(thresh_raw, contours, cv::RETR_EXTERNAL, cv::CHAIN_APPROX_SIMPLE);

    cv::Mat thresh = cv::Mat::zeros(thresh_raw.size(), CV_8UC1);

    for (const auto& contour : contours) {
        cv::RotatedRect rect = cv::minAreaRect(contour);
        float width = rect.size.width;
        float height = rect.size.height;
        float longer = std::max(width, height);
        float shorter = std::min(width, height);
        float aspect_ratio = longer / std::max(shorter, 1.0f);

        bool is_long = longer > config.min_contour_length;
        bool is_elongated = aspect_ratio > config.min_aspect_ratio;

        if (is_long || is_elongated) {
            cv::drawContours(thresh, std::vector<std::vector<cv::Point>>{contour}, 0, 255, cv::FILLED);
        }
    }

    return thresh;
}

void warp_birdseye(const cv::Mat& frame, const PipelineConfig& config, cv::Mat& out) {
    float w = static_cast<float>(frame.cols);
    float h = static_cast<float>(frame.rows);

    cv::Point2f src[4] = {
        {config.warp_src_top_left_x * w,     config.warp_src_top_left_y * h},
        {config.warp_src_top_right_x * w,    config.warp_src_top_right_y * h},
        {config.warp_src_bottom_right_x * w, config.warp_src_bottom_right_y * h},
        {config.warp_src_bottom_left_x * w,  config.warp_src_bottom_left_y * h},
    };

    cv::Point2f dst[4] = {
        {config.warp_dst_top_left_x * w,     config.warp_dst_top_left_y * h},
        {config.warp_dst_top_right_x * w,    config.warp_dst_top_right_y * h},
        {config.warp_dst_bottom_right_x * w, config.warp_dst_bottom_right_y * h},
        {config.warp_dst_bottom_left_x * w,  config.warp_dst_bottom_left_y * h},
    };

    cv::Mat M = cv::getPerspectiveTransform(src, dst);
    cv::warpPerspective(frame, out, M, frame.size());
}

}  // namespace nsm
