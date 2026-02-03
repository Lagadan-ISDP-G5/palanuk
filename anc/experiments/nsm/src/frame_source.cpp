#include "frame_source.h"
#include <opencv2/imgcodecs.hpp>
#include <algorithm>
#include <iostream>

#include <iox2/service_name.hpp>

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

// IceoryxSource implementation
IceoryxSource::IceoryxSource(const std::string& service_name)
    : service_name_(service_name) {}

IceoryxSource::~IceoryxSource() {
    release();
}

bool IceoryxSource::open() {
    using namespace iox2;

    auto node_result = NodeBuilder().create<ServiceType::Ipc>();
    if (!node_result.has_value()) {
        std::cerr << "IceoryxSource: Failed to create iceoryx2 node" << std::endl;
        return false;
    }
    node_.emplace(std::move(node_result.value()));

    auto service_name = ServiceName::create(service_name_.c_str());
    if (!service_name.has_value()) {
        std::cerr << "IceoryxSource: Failed to create service name '" << service_name_ << "'" << std::endl;
        node_.reset();
        return false;
    }

    auto service = node_->service_builder(service_name.value())
        .publish_subscribe<IpcFrame>()
        .open_or_create();
    if (!service.has_value()) {
        std::cerr << "IceoryxSource: Failed to open/create service '" << service_name_ << "'" << std::endl;
        node_.reset();
        return false;
    }

    auto sub = service.value().subscriber_builder().create();
    if (!sub.has_value()) {
        std::cerr << "IceoryxSource: Failed to create subscriber for '" << service_name_ << "'" << std::endl;
        node_.reset();
        return false;
    }
    subscriber_.emplace(std::move(sub.value()));

    std::cout << "IceoryxSource: Connected to service '" << service_name_ << "'" << std::endl;
    return true;
}

bool IceoryxSource::read(cv::Mat& frame) {
    if (!subscriber_.has_value()) {
        return false;
    }

    auto receive_result = subscriber_->receive();
    if (!receive_result.has_value()) {
        std::cerr << "IceoryxSource: receive error" << std::endl;
        return false;
    }

    auto& sample_opt = receive_result.value();
    if (!sample_opt.has_value()) {
        // No sample available (not an error, just no data yet)
        return false;
    }

    const IpcFrame& ipc_frame = sample_opt.value().payload();

    // Debug: print what we received
    std::cerr << "IceoryxSource: received frame seq=" << ipc_frame.sequence
              << " w=" << ipc_frame.width << " h=" << ipc_frame.height
              << " stride=" << ipc_frame.stride << " len=" << ipc_frame.len
              << " format=" << static_cast<uint32_t>(ipc_frame.format) << std::endl;

    // Validate frame data
    if (ipc_frame.len == 0 || ipc_frame.width == 0 || ipc_frame.height == 0) {
        std::cerr << "IceoryxSource: validation failed" << std::endl;
        return false;
    }

    // Convert YUV to BGR for OpenCV based on pixel format
    cv::Mat yuv(static_cast<int>(ipc_frame.height + ipc_frame.height / 2),
                static_cast<int>(ipc_frame.width), CV_8UC1,
                const_cast<uint8_t*>(ipc_frame.data));

    switch (ipc_frame.format) {
        case PixelFormat::Yuv420:
            cv::cvtColor(yuv, frame, cv::COLOR_YUV2BGR_I420);
            break;
        case PixelFormat::Nv12:
            cv::cvtColor(yuv, frame, cv::COLOR_YUV2BGR_NV12);
            break;
        case PixelFormat::Nv21:
            cv::cvtColor(yuv, frame, cv::COLOR_YUV2BGR_NV21);
            break;
        default:
            cv::cvtColor(yuv, frame, cv::COLOR_YUV2BGR_I420);
            break;
    }

    last_sequence_ = ipc_frame.sequence;
    last_timestamp_ns_ = ipc_frame.timestamp_ns;

    return true;
}

bool IceoryxSource::isOpened() const {
    return subscriber_.has_value();
}

void IceoryxSource::release() {
    subscriber_.reset();
    node_.reset();
}

std::string IceoryxSource::getName() const {
    return "iox:" + service_name_;
}

// Factory function
std::unique_ptr<FrameSource> createFrameSource(const std::string& source) {
    // Check if it's an iceoryx2 service
    if (source.find("iox:") == 0) {
        std::string service_name = source.substr(4);
        return std::make_unique<IceoryxSource>(service_name);
    }

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
