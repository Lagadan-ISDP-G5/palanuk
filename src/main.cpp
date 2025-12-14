#include <opencv2/opencv.hpp>
#include <filesystem>
#include <iostream>
#include <vector>

namespace fs = std::filesystem;

struct LineDetectionResult {
    std::vector<cv::Point2f> points;
    cv::Vec4f fitted_line;  // (vx, vy, x0, y0)
    bool valid = false;
};

cv::Mat threshold_white_line(const cv::Mat& img) {
    cv::Mat gray, blurred, thresh;
    cv::cvtColor(img, gray, cv::COLOR_BGR2GRAY);
    cv::GaussianBlur(gray, blurred, cv::Size(5, 5), 0);
    cv::threshold(blurred, thresh, 200, 255, cv::THRESH_BINARY);
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

cv::Mat visualize_result(const cv::Mat& original, const cv::Mat& thresh, const LineDetectionResult& result) {
    cv::Mat vis = original.clone();

    // Draw detected points
    for (const auto& pt : result.points) {
        cv::circle(vis, pt, 8, cv::Scalar(0, 255, 0), -1);  // green filled circles
        cv::circle(vis, pt, 8, cv::Scalar(0, 0, 0), 2);     // black outline
    }

    // Draw fitted line
    if (result.valid) {
        float vx = result.fitted_line[0];
        float vy = result.fitted_line[1];
        float x0 = result.fitted_line[2];
        float y0 = result.fitted_line[3];

        // Extend line to image boundaries
        int y1 = vis.rows;
        int y2 = 0;
        int x1 = static_cast<int>(x0 + (y1 - y0) * vx / vy);
        int x2 = static_cast<int>(x0 + (y2 - y0) * vx / vy);

        cv::line(vis, cv::Point(x1, y1), cv::Point(x2, y2), cv::Scalar(0, 0, 255), 3);  // red line
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
        LineDetectionResult result = detect_line_sliding_window(thresh);

        std::cout << " -> " << result.points.size() << " points";
        if (result.valid) {
            std::cout << ", line fitted";
        }
        std::cout << std::endl;

        // Save visualization
        cv::Mat output = visualize_result(img, thresh, result);
        fs::path output_path = output_dir / ("processed_" + filename);
        cv::imwrite(output_path.string(), output);
    }

    std::cout << "\nProcessed images saved to " << output_dir << std::endl;
    return 0;
}
