#include "frame_source.h"
#include <opencv2/imgcodecs.hpp>
#include <algorithm>
#include <iostream>

namespace fs = std::filesystem;

namespace nsm {

// CameraSource implementation
CameraSource::CameraSource(int device_id, int width, int height)
    : device_id_(device_id), width_(width), height_(height) {}

bool CameraSource::open() {
    cap_.open(device_id_);
    if (cap_.isOpened()) {
        cap_.set(cv::CAP_PROP_FRAME_WIDTH, width_);
        cap_.set(cv::CAP_PROP_FRAME_HEIGHT, height_);
        return true;
    }
    return false;
}

bool CameraSource::read(cv::Mat& frame) {
    return cap_.read(frame);
}

bool CameraSource::isOpened() const {
    return cap_.isOpened();
}

void CameraSource::release() {
    cap_.release();
}

std::string CameraSource::getName() const {
    return "camera:" + std::to_string(device_id_);
}

// VideoFileSource implementation
VideoFileSource::VideoFileSource(const std::string& path, bool loop)
    : path_(path), loop_(loop) {}

bool VideoFileSource::open() {
    return cap_.open(path_);
}

bool VideoFileSource::read(cv::Mat& frame) {
    bool success = cap_.read(frame);
    if (!success && loop_) {
        cap_.set(cv::CAP_PROP_POS_FRAMES, 0);
        success = cap_.read(frame);
    }
    return success;
}

bool VideoFileSource::isOpened() const {
    return cap_.isOpened();
}

void VideoFileSource::release() {
    cap_.release();
}

std::string VideoFileSource::getName() const {
    return fs::path(path_).filename().string();
}

// ImageDirectorySource implementation
ImageDirectorySource::ImageDirectorySource(const std::string& directory)
    : directory_(directory) {}

bool ImageDirectorySource::open() {
    if (!fs::exists(directory_) || !fs::is_directory(directory_)) {
        return false;
    }

    files_.clear();
    for (const auto& entry : fs::directory_iterator(directory_)) {
        if (!entry.is_regular_file()) continue;

        std::string ext = entry.path().extension().string();
        std::transform(ext.begin(), ext.end(), ext.begin(), ::tolower);

        if (ext == ".jpg" || ext == ".jpeg" || ext == ".png" || ext == ".bmp") {
            files_.push_back(entry.path());
        }
    }

    std::sort(files_.begin(), files_.end());
    current_index_ = 0;
    return !files_.empty();
}

bool ImageDirectorySource::read(cv::Mat& frame) {
    if (current_index_ >= files_.size()) {
        return false;
    }

    current_filename_ = files_[current_index_].filename().string();
    frame = cv::imread(files_[current_index_].string());
    current_index_++;

    return !frame.empty();
}

bool ImageDirectorySource::isOpened() const {
    return !files_.empty() && current_index_ < files_.size();
}

void ImageDirectorySource::release() {
    files_.clear();
    current_index_ = 0;
}

std::string ImageDirectorySource::getName() const {
    return directory_;
}

std::string ImageDirectorySource::getCurrentFilename() const {
    return current_filename_;
}

// Factory function
std::unique_ptr<FrameSource> createFrameSource(const std::string& source) {
    // Check if it's a camera index
    if (source.find("camera:") == 0) {
        int id = std::stoi(source.substr(7));
        return std::make_unique<CameraSource>(id);
    }

    // Check if it's a number (camera id)
    try {
        int id = std::stoi(source);
        return std::make_unique<CameraSource>(id);
    } catch (...) {}

    // Check if it's a directory
    if (fs::is_directory(source)) {
        return std::make_unique<ImageDirectorySource>(source);
    }

    // Assume it's a video file
    if (fs::exists(source)) {
        return std::make_unique<VideoFileSource>(source);
    }

    return nullptr;
}

}  // namespace nsm
