#include "visualization.h"
#include <opencv2/imgproc.hpp>

namespace nsm {

cv::Mat visualize_result(const cv::Mat& original, const FrameResult& result) {
    cv::Mat vis = original.clone();

    // Draw center line detected points
    for (const auto& pt : result.center_line.points) {
        cv::circle(vis, pt, 8, cv::Scalar(0, 255, 0), -1);
        cv::circle(vis, pt, 8, cv::Scalar(0, 0, 0), 2);
    }

    // Draw fitted center line
    if (result.center_line.valid) {
        float vx = result.center_line.fitted_line[0];
        float vy = result.center_line.fitted_line[1];
        float x0 = result.center_line.fitted_line[2];
        float y0 = result.center_line.fitted_line[3];

        int y1 = vis.rows;
        int y2 = 0;
        int x1 = static_cast<int>(x0 + (y1 - y0) * vx / vy);
        int x2 = static_cast<int>(x0 + (y2 - y0) * vx / vy);

        cv::line(vis, cv::Point(x1, y1), cv::Point(x2, y2), cv::Scalar(0, 0, 255), 3);
    }

    // Draw L-corner if detected
    if (result.corner.detected) {
        cv::circle(vis, result.corner.corner_point, 15, cv::Scalar(255, 255, 0), -1);
        cv::circle(vis, result.corner.corner_point, 15, cv::Scalar(0, 0, 0), 3);

        cv::Point2f arrow_end = result.corner.corner_point + result.corner.horizontal_direction * 80;
        cv::arrowedLine(vis, result.corner.corner_point, arrow_end,
                        cv::Scalar(255, 0, 255), 3, cv::LINE_AA, 0, 0.3);

        cv::putText(vis, "CORNER", result.corner.corner_point + cv::Point2f(-30, -25),
                    cv::FONT_HERSHEY_SIMPLEX, 0.7, cv::Scalar(255, 255, 0), 2);
    }

    return vis;
}

cv::Mat visualize_side_by_side(const cv::Mat& original, const FrameResult& result) {
    cv::Mat vis = visualize_result(original, result);

    cv::Mat thresh_color;
    cv::cvtColor(result.thresholded, thresh_color, cv::COLOR_GRAY2BGR);

    cv::Mat combined;
    cv::hconcat(thresh_color, vis, combined);

    return combined;
}

}  // namespace nsm
