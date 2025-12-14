#ifndef NSM_FRAME_SOURCE_H
#define NSM_FRAME_SOURCE_H

#include <opencv2/core.hpp>
#include <opencv2/videoio.hpp>
#include <filesystem>
#include <memory>
#include <string>
#include <vector>

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

std::unique_ptr<FrameSource> createFrameSource(const std::string& source);

}  // namespace nsm

#endif  // NSM_FRAME_SOURCE_H
