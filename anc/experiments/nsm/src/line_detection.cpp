#include "line_detection.h"
#include <opencv2/imgproc.hpp>
#include <algorithm>

namespace nsm {

void detect_line_sliding_window(const cv::Mat& thresh, const PipelineConfig& config, LineDetectionResult& out) {
    out.reset();

    int height = thresh.rows;
    int width = thresh.cols;
    int window_height = height / config.num_windows;

    // Find initial x position from bottom strip
    cv::Mat bottom_strip = thresh(cv::Rect(0, height - window_height, width, window_height));
    cv::Moments m = cv::moments(bottom_strip, true);
    if (m.m00 == 0) return;

    int current_x = static_cast<int>(m.m10 / m.m00);

    for (int i = 0; i < config.num_windows; i++) {
        int y_top = height - (i + 1) * window_height;
        int y_center = y_top + window_height / 2;

        int x_left = std::max(0, current_x - config.window_width / 2);
        int x_right = std::min(width, current_x + config.window_width / 2);
        int rect_width = x_right - x_left;

        if (rect_width <= 0 || y_top < 0) break;

        cv::Rect window_rect(x_left, y_top, rect_width, window_height);
        cv::Mat window = thresh(window_rect);

        cv::Moments wm = cv::moments(window, true);
        if (wm.m00 > config.min_pixel_threshold) {
            int local_x = static_cast<int>(wm.m10 / wm.m00);
            current_x = x_left + local_x;
            out.points.emplace_back(current_x, y_center);
        }
    }

    if (out.points.size() >= 2) {
        cv::fitLine(out.points, out.fitted_line, cv::DIST_L2, 0, 0.01, 0.01);
        out.valid = true;
    }
}

void detect_horizontal_line(const cv::Mat& thresh, int start_y, const PipelineConfig& config, LineDetectionResult& out) {
    out.reset();

    int height = thresh.rows;
    int width = thresh.cols;
    int window_width = width / config.horiz_num_windows;

    int search_top = std::max(0, start_y - config.horiz_window_height);
    int search_bottom = std::min(height, start_y + config.horiz_window_height);
    int search_height = search_bottom - search_top;

    if (search_height <= 0) return;

    int current_y = start_y;

    for (int i = 0; i < config.horiz_num_windows; i++) {
        int x_left = i * window_width;
        int x_center = x_left + window_width / 2;

        int y_top = std::max(0, current_y - config.horiz_window_height / 2);
        int y_bottom = std::min(height, current_y + config.horiz_window_height / 2);
        int rect_height = y_bottom - y_top;

        if (rect_height <= 0 || x_left + window_width > width) break;

        cv::Rect window_rect(x_left, y_top, window_width, rect_height);
        cv::Mat window = thresh(window_rect);

        cv::Moments wm = cv::moments(window, true);
        if (wm.m00 > config.min_pixel_threshold) {
            int local_y = static_cast<int>(wm.m01 / wm.m00);
            current_y = y_top + local_y;
            out.points.emplace_back(x_center, current_y);
        }
    }

    if (out.points.size() >= 3) {
        cv::fitLine(out.points, out.fitted_line, cv::DIST_L2, 0, 0.01, 0.01);

        float vx = std::abs(out.fitted_line[0]);
        float vy = std::abs(out.fitted_line[1]);
        bool is_horizontal = vx > vy * 2; // fitted line twice as long as it is wide ~= it's horizontal

        float min_x = out.points.front().x;
        float max_x = out.points.back().x;
        bool has_span = (max_x - min_x) > 50;

        out.valid = is_horizontal && has_span;
    }
}

float get_line_center_offset(const LineDetectionResult& result, int frame_width) {
    if (result.points.empty() || frame_width <= 0) {
        return 0.0f;
    }

    // Points are ordered bottom-to-top, so first points are closest to bottom
    size_t num_points = std::min(result.points.size(), size_t(3));
    float sum_x = 0.0f;
    for (size_t i = 0; i < num_points; i++) {
        sum_x += result.points[i].x;
    }
    float avg_x = sum_x / num_points;

    // Normalize: 0 = center, -1 = left edge, +1 = right edge
    float center = frame_width / 2.0f;
    return (avg_x - center) / center;
}

}  // namespace nsm
