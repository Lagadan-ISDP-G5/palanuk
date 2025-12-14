#include <opencv2/opencv.hpp>
#include <iostream>

int main(int argc, char** argv) {
    std::cout << "OpenCV version: " << CV_VERSION << std::endl;

    // Create a simple test image to verify OpenCV works
    cv::Mat img(100, 100, CV_8UC3, cv::Scalar(255, 0, 0));
    std::cout << "Created test image: " << img.cols << "x" << img.rows << std::endl;

    return 0;
}
