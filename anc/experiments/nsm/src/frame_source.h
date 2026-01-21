#ifndef NSM_FRAME_SOURCE_H
#define NSM_FRAME_SOURCE_H

#include <opencv2/core.hpp>
#include <opencv2/videoio.hpp>
#include <opencv2/imgproc.hpp>
#include <filesystem>
#include <memory>
#include <string>
#include <vector>
#include <cstdint>

// Uncomment when iceoryx2-ffi-c is available
// extern "C" {
// #include <iox2/iceoryx2.h>
// }

namespace nsm {

class FrameSource {
public:
    virtual ~FrameSource() = default;
    virtual bool open() = 0;
    virtual bool read(cv::Mat& frame) = 0;
    virtual bool isOpened() const = 0;
    virtual void release() = 0;
    virtual std::string getName() const = 0;
};

class CameraSource : public FrameSource {
public:
    explicit CameraSource(int device_id = 0, int width = 640, int height = 480);
    bool open() override;
    bool read(cv::Mat& frame) override;
    bool isOpened() const override;
    void release() override;
    std::string getName() const override;

private:
    cv::VideoCapture cap_;
    int device_id_;
    int width_;
    int height_;
};

class VideoFileSource : public FrameSource {
public:
    explicit VideoFileSource(const std::string& path, bool loop = false);
    bool open() override;
    bool read(cv::Mat& frame) override;
    bool isOpened() const override;
    void release() override;
    std::string getName() const override;

private:
    cv::VideoCapture cap_;
    std::string path_;
    bool loop_;
};

class ImageDirectorySource : public FrameSource {
public:
    explicit ImageDirectorySource(const std::string& directory);
    bool open() override;
    bool read(cv::Mat& frame) override;
    bool isOpened() const override;
    void release() override;
    std::string getName() const override;
    std::string getCurrentFilename() const;

private:
    std::string directory_;
    std::vector<std::filesystem::path> files_;
    size_t current_index_ = 0;
    std::string current_filename_;
};

// Frame layout for iceoryx2 IPC - must match Rust's #[repr(C)] struct
constexpr size_t MAX_FRAME_SIZE = 1920 * 1080 * 3 / 2;  // ~3.1MB for 1080p YUV420

struct IpcFrame {
    uint64_t timestamp_ns;
    uint64_t sequence;
    uint32_t width;
    uint32_t height;
    uint32_t stride;
    uint32_t len;
    uint8_t data[MAX_FRAME_SIZE];
};

// Shared memory frame source using iceoryx2
// Usage: createFrameSource("iox:camera/frames")
class IceoryxSource : public FrameSource {
public:
    explicit IceoryxSource(const std::string& service_name);
    ~IceoryxSource() override;
    bool open() override;
    bool read(cv::Mat& frame) override;
    bool isOpened() const override;
    void release() override;
    std::string getName() const override;
    uint64_t getLastSequence() const { return last_sequence_; }
    uint64_t getLastTimestamp() const { return last_timestamp_ns_; }

private:
    std::string service_name_;
    bool opened_ = false;
    uint64_t last_sequence_ = 0;
    uint64_t last_timestamp_ns_ = 0;

    // iceoryx2 handles (uncomment when iceoryx2-ffi-c is available)
    // iox2_node_h node_ = nullptr;
    // iox2_subscriber_h subscriber_ = nullptr;
};

std::unique_ptr<FrameSource> createFrameSource(const std::string& source);

}  // namespace nsm

#endif  // NSM_FRAME_SOURCE_H
