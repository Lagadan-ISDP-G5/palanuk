#include "corner_detection.h"
#include "line_detection.h"
#include <opencv2/imgproc.hpp>
#include <limits>

namespace nsm {

void detect_harris_corners(const cv::Mat& thresh, const PipelineConfig& config, std::vector<cv::Point2f>& out) {
    out.clear();

    cv::Mat corners_response;
    cv::cornerHarris(thresh, corners_response, config.harris_block_size, config.harris_ksize, config.harris_k);

    cv::Mat corners_norm;
    cv::normalize(corners_response, corners_norm, 0, 255, cv::NORM_MINMAX);

    // TODO: these hardcoded integers should probably match the config.max_corners field value
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
                    out.emplace_back(x, y);
                    if (out.size() >= static_cast<size_t>(config.max_corners)) {
                        return;
                    }
                }
            }
        }
    }
}

void detect_L_corner(const cv::Mat& thresh, const LineDetectionResult& center_line, const PipelineConfig& config,
                     LineDetectionResult& horiz_scratch, std::vector<cv::Point2f>& harris_scratch,
                     CornerDetectionResult& out) {
    out.reset();

    if (!center_line.valid || center_line.points.size() < 2) {
        return;
    }

    cv::Point2f endpoint = center_line.points.back();
    detect_horizontal_line(thresh, static_cast<int>(endpoint.y), config, horiz_scratch);
    detect_harris_corners(thresh, config, harris_scratch);

    cv::Point2f best_corner = endpoint;
    float best_dist = std::numeric_limits<float>::max();

    for (const auto& corner : harris_scratch) {
        float dist = cv::norm(corner - endpoint);
        if (dist < best_dist && dist < config.corner_max_distance) {
            best_dist = dist;
            best_corner = corner;
        }
    }

    if (best_dist < config.corner_max_distance && horiz_scratch.valid) {
        out.corner_point = best_corner;
        out.detected = true;

        out.horizontal_direction = cv::Point2f(horiz_scratch.fitted_line[0], horiz_scratch.fitted_line[1]);
        // this removes corner direction info by assuming everything is rightwards
        // which is not a bad assertion, because you can literally only turn right in the track,
        // but still important to document
        // e.g. (1.0, 0.0) -> perfectly right
        // (-1.0, 0.0) -> perfectly left
        if (out.horizontal_direction.x < 0) {
            out.horizontal_direction = -out.horizontal_direction;
        }
    }
}

}  // namespace nsm
