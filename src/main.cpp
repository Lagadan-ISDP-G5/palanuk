#include <opencv2/opencv.hpp>
#include <filesystem>
#include <iostream>
#include <vector>
#include <limits>

namespace fs = std::filesystem;

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

// ROI: bottom 47% of frame (top 53% is ignored)
constexpr float ROI_IGNORE_TOP_PERCENT = 0.53f;

cv::Mat threshold_white_line(const cv::Mat& img) {
    cv::Mat gray, blurred, thresh_raw;
    cv::cvtColor(img, gray, cv::COLOR_BGR2GRAY);
    cv::GaussianBlur(gray, blurred, cv::Size(5, 5), 0);
    cv::threshold(blurred, thresh_raw, 200, 255, cv::THRESH_BINARY);

    // Mask out the top 53% of the frame
    int roi_top = static_cast<int>(img.rows * ROI_IGNORE_TOP_PERCENT);
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

        // Lines are long (longer dimension > 50px) OR elongated (aspect > 2.5)
        // Glare is short and round
        bool is_long = longer > 50;
        bool is_elongated = aspect_ratio > 2.5;

        if (is_long || is_elongated) {
            cv::drawContours(thresh, std::vector<std::vector<cv::Point>>{contour}, 0, 255, cv::FILLED);
        }
    }

    return thresh;
}

LineDetectionResult detect_line_sliding_window(const cv::Mat& thresh, int num_windows = 10, int window_width = 100) {
    LineDetectionResult result;
    int height = thresh.rows;
    int width = thresh.cols;
    int window_height = height / num_windows;

    // Find initial x position from bottom strip
    cv::Mat bottom_strip = thresh(cv::Rect(0, height - window_height, width, window_height));
    cv::Moments m = cv::moments(bottom_strip, true);
    if (m.m00 == 0) return result;

    int current_x = static_cast<int>(m.m10 / m.m00);

    for (int i = 0; i < num_windows; i++) {
        int y_top = height - (i + 1) * window_height;
        int y_center = y_top + window_height / 2;

        int x_left = std::max(0, current_x - window_width / 2);
        int x_right = std::min(width, current_x + window_width / 2);
        int rect_width = x_right - x_left;

        if (rect_width <= 0 || y_top < 0) break;

        cv::Rect window_rect(x_left, y_top, rect_width, window_height);
        cv::Mat window = thresh(window_rect);

        cv::Moments wm = cv::moments(window, true);
        if (wm.m00 > 50) {  // minimum pixel threshold
            int local_x = static_cast<int>(wm.m10 / wm.m00);
            current_x = x_left + local_x;
            result.points.emplace_back(current_x, y_center);
        }
    }

    if (result.points.size() >= 2) {
        cv::fitLine(result.points, result.fitted_line, cv::DIST_L2, 0, 0.01, 0.01);
        result.valid = true;
    }

    return result;
}

// Detect horizontal line using sliding windows (scans left-to-right)
LineDetectionResult detect_horizontal_line(const cv::Mat& thresh, int start_y, int num_windows = 10, int window_height = 40) {
    LineDetectionResult result;
    int height = thresh.rows;
    int width = thresh.cols;
    int window_width = width / num_windows;

    // Search region around the expected y position
    int search_top = std::max(0, start_y - window_height);
    int search_bottom = std::min(height, start_y + window_height);
    int search_height = search_bottom - search_top;

    if (search_height <= 0) return result;

    int current_y = start_y;

    for (int i = 0; i < num_windows; i++) {
        int x_left = i * window_width;
        int x_center = x_left + window_width / 2;

        int y_top = std::max(0, current_y - window_height / 2);
        int y_bottom = std::min(height, current_y + window_height / 2);
        int rect_height = y_bottom - y_top;

        if (rect_height <= 0 || x_left + window_width > width) break;

        cv::Rect window_rect(x_left, y_top, window_width, rect_height);
        cv::Mat window = thresh(window_rect);

        cv::Moments wm = cv::moments(window, true);
        if (wm.m00 > 50) {
            int local_y = static_cast<int>(wm.m01 / wm.m00);
            current_y = y_top + local_y;
            result.points.emplace_back(x_center, current_y);
        }
    }

    if (result.points.size() >= 2) {
        cv::fitLine(result.points, result.fitted_line, cv::DIST_L2, 0, 0.01, 0.01);
        result.valid = true;
    }

    return result;
}

// Detect L-corner using Harris corner detection on the thresholded image
std::vector<cv::Point2f> detect_harris_corners(const cv::Mat& thresh, int max_corners = 10) {
    std::vector<cv::Point2f> corners;

    cv::Mat corners_response;
    cv::cornerHarris(thresh, corners_response, 9, 3, 0.04);

    cv::Mat corners_norm;
    cv::normalize(corners_response, corners_norm, 0, 255, cv::NORM_MINMAX);

    // Find local maxima above threshold
    double threshold_val = 150;
    for (int y = 10; y < corners_norm.rows - 10; y++) {
        for (int x = 10; x < corners_norm.cols - 10; x++) {
            float val = corners_norm.at<float>(y, x);
            if (val > threshold_val) {
                // Check if local maximum
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
                    if (corners.size() >= static_cast<size_t>(max_corners)) {
                        return corners;
                    }
                }
            }
        }
    }

    return corners;
}

// Main corner detection: finds where center line meets horizontal line
CornerDetectionResult detect_L_corner(const cv::Mat& thresh, const LineDetectionResult& center_line) {
    CornerDetectionResult result;

    if (!center_line.valid || center_line.points.size() < 2) {
        return result;
    }

    // The corner is likely near the last detected point in the sliding window sequence
    // (where the line ends or changes direction)
    cv::Point2f endpoint = center_line.points.back();

    // Search for horizontal line near the endpoint
    LineDetectionResult horiz = detect_horizontal_line(thresh, static_cast<int>(endpoint.y));

    // Also use Harris corners as candidates
    std::vector<cv::Point2f> harris_corners = detect_harris_corners(thresh);

    // Find the best corner candidate near the endpoint
    cv::Point2f best_corner = endpoint;
    float best_dist = std::numeric_limits<float>::max();

    for (const auto& corner : harris_corners) {
        // Corner should be near the endpoint of the center line
        float dist = cv::norm(corner - endpoint);
        if (dist < best_dist && dist < 100) {  // within 100 pixels
            best_dist = dist;
            best_corner = corner;
        }
    }

    // Require both a nearby Harris corner AND a valid horizontal line
    if (best_dist < 100 && horiz.valid) {
        result.corner_point = best_corner;
        result.detected = true;

        // Determine horizontal direction from horizontal line if available
        if (horiz.valid) {
            result.horizontal_direction = cv::Point2f(horiz.fitted_line[0], horiz.fitted_line[1]);
            // Ensure pointing right (positive x)
            if (result.horizontal_direction.x < 0) {
                result.horizontal_direction = -result.horizontal_direction;
            }
        } else {
            result.horizontal_direction = cv::Point2f(1, 0);  // default right
        }
    }

    return result;
}

cv::Mat visualize_result(const cv::Mat& original, const cv::Mat& thresh,
                         const LineDetectionResult& center_line,
                         const CornerDetectionResult& corner) {
    cv::Mat vis = original.clone();

    // Draw center line detected points
    for (const auto& pt : center_line.points) {
        cv::circle(vis, pt, 8, cv::Scalar(0, 255, 0), -1);  // green filled circles
        cv::circle(vis, pt, 8, cv::Scalar(0, 0, 0), 2);     // black outline
    }

    // Draw fitted center line
    if (center_line.valid) {
        float vx = center_line.fitted_line[0];
        float vy = center_line.fitted_line[1];
        float x0 = center_line.fitted_line[2];
        float y0 = center_line.fitted_line[3];

        // Extend line to image boundaries
        int y1 = vis.rows;
        int y2 = 0;
        int x1 = static_cast<int>(x0 + (y1 - y0) * vx / vy);
        int x2 = static_cast<int>(x0 + (y2 - y0) * vx / vy);

        cv::line(vis, cv::Point(x1, y1), cv::Point(x2, y2), cv::Scalar(0, 0, 255), 3);  // red line
    }

    // Draw L-corner if detected
    if (corner.detected) {
        // Draw corner point (large cyan circle)
        cv::circle(vis, corner.corner_point, 15, cv::Scalar(255, 255, 0), -1);  // cyan filled
        cv::circle(vis, corner.corner_point, 15, cv::Scalar(0, 0, 0), 3);       // black outline

        // Draw horizontal direction arrow
        cv::Point2f arrow_end = corner.corner_point + corner.horizontal_direction * 80;
        cv::arrowedLine(vis, corner.corner_point, arrow_end,
                        cv::Scalar(255, 0, 255), 3, cv::LINE_AA, 0, 0.3);  // magenta arrow

        // Label the corner
        cv::putText(vis, "CORNER", corner.corner_point + cv::Point2f(-30, -25),
                    cv::FONT_HERSHEY_SIMPLEX, 0.7, cv::Scalar(255, 255, 0), 2);
    }

    // Create side-by-side output: threshold | visualization
    cv::Mat thresh_color;
    cv::cvtColor(thresh, thresh_color, cv::COLOR_GRAY2BGR);

    cv::Mat combined;
    cv::hconcat(thresh_color, vis, combined);

    return combined;
}

int main(int argc, char** argv) {
    std::cout << "OpenCV version: " << CV_VERSION << std::endl;

    fs::path stills_dir = "../data/stills";
    fs::path output_dir = "../data/processed";

    if (!fs::exists(stills_dir)) {
        std::cerr << "Error: " << stills_dir << " not found" << std::endl;
        return 1;
    }

    fs::create_directories(output_dir);

    for (const auto& entry : fs::directory_iterator(stills_dir)) {
        if (!entry.is_regular_file()) continue;

        // Skip non-image files
        std::string ext = entry.path().extension().string();
        if (ext != ".jpg" && ext != ".jpeg" && ext != ".png" && ext != ".bmp") continue;

        std::string path = entry.path().string();
        cv::Mat img = cv::imread(path);

        if (img.empty()) {
            std::cerr << "Failed to load: " << path << std::endl;
            continue;
        }

        std::string filename = entry.path().filename().string();
        std::cout << filename << ": " << img.cols << "x" << img.rows;

        // Process
        cv::Mat thresh = threshold_white_line(img);
        LineDetectionResult center_line = detect_line_sliding_window(thresh);
        CornerDetectionResult corner = detect_L_corner(thresh, center_line);

        std::cout << " -> " << center_line.points.size() << " points";
        if (center_line.valid) {
            std::cout << ", line fitted";
        }
        if (corner.detected) {
            std::cout << ", CORNER at (" << static_cast<int>(corner.corner_point.x)
                      << "," << static_cast<int>(corner.corner_point.y) << ")";
        }
        std::cout << std::endl;

        // Save visualization
        cv::Mat output = visualize_result(img, thresh, center_line, corner);
        fs::path output_path = output_dir / ("processed_" + filename);
        cv::imwrite(output_path.string(), output);
    }

    std::cout << "\nProcessed images saved to " << output_dir << std::endl;
    return 0;
}
