#include "corner_detection.h"
#include "line_detection.h"
#include <opencv2/imgproc.hpp>
#include <limits>

namespace nsm {

std::vector<cv::Point2f> detect_harris_corners(const cv::Mat& thresh, const PipelineConfig& config) {
    std::vector<cv::Point2f> corners;

    cv::Mat corners_response;
    cv::cornerHarris(thresh, corners_response, config.harris_block_size, config.harris_ksize, config.harris_k);

    cv::Mat corners_norm;
    cv::normalize(corners_response, corners_norm, 0, 255, cv::NORM_MINMAX);

    for (int y = 10; y < corners_norm.rows - 10; y++) {
        for (int x = 10; x < corners_norm.cols - 10; x++) {
            float val = corners_norm.at<float>(y, x);
            if (val > config.harris_threshold) {
                bool is_max = true;
                for (int dy = -5; dy <= 5 && is_max; dy++) {
                    for (int dx = -5; dx <= 5 && is_max; dx++) {
                        if (dx == 0 && dy == 0) continue;
                        if (corners_norm.at<float>(y + dy, x + dx) >= val) {
                            is_max = false;
                        }
                    }
                }
                if (is_max) {
                    corners.emplace_back(x, y);
                    if (corners.size() >= static_cast<size_t>(config.max_corners)) {
                        return corners;
                    }
                }
            }
        }
    }

    return corners;
}

CornerDetectionResult detect_L_corner(const cv::Mat& thresh, const LineDetectionResult& center_line, const PipelineConfig& config) {
    CornerDetectionResult result;

    if (!center_line.valid || center_line.points.size() < 2) {
        return result;
    }

    cv::Point2f endpoint = center_line.points.back();
    LineDetectionResult horiz = detect_horizontal_line(thresh, static_cast<int>(endpoint.y), config);
    std::vector<cv::Point2f> harris_corners = detect_harris_corners(thresh, config);

    cv::Point2f best_corner = endpoint;
    float best_dist = std::numeric_limits<float>::max();

    for (const auto& corner : harris_corners) {
        float dist = cv::norm(corner - endpoint);
        if (dist < best_dist && dist < config.corner_max_distance) {
            best_dist = dist;
            best_corner = corner;
        }
    }

    if (best_dist < config.corner_max_distance && horiz.valid) {
        result.corner_point = best_corner;
        result.detected = true;

        if (horiz.valid) {
            result.horizontal_direction = cv::Point2f(horiz.fitted_line[0], horiz.fitted_line[1]);
            if (result.horizontal_direction.x < 0) {
                result.horizontal_direction = -result.horizontal_direction;
            }
        } else {
            result.horizontal_direction = cv::Point2f(1, 0);
        }
    }

    return result;
}

}  // namespace nsm
