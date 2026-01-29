#include <opencv2/opencv.hpp>
#include <filesystem>
#include <iostream>
#include <string>

#include "types.h"
#include "pipeline.h"
#include "frame_source.h"
#include "visualization.h"
#include "line_detection.h"

namespace fs = std::filesystem;

void printUsage(const char* program) {
    std::cout << "Usage: " << program << " [OPTIONS] [SOURCE]\n"
              << "\nSOURCE can be:\n"
              << "  <directory>    Process all images in directory\n"
              << "  <video_file>   Process video file\n"
              << "  <camera_id>    Use camera (0, 1, etc.)\n"
              << "  camera:<id>    Use camera explicitly\n"
              << "\nOPTIONS:\n"
              << "  --headless     Run without display (batch mode only)\n"
              << "  --output DIR   Output directory for processed images\n"
              << "  --help         Show this help\n"
              << "\nExamples:\n"
              << "  " << program << " ../data/stills\n"
              << "  " << program << " 0\n"
              << "  " << program << " recording.mp4\n";
}

int runBatchMode(nsm::ImageDirectorySource& source, nsm::Pipeline& pipeline,
                 const std::string& output_dir, bool headless) {
    fs::create_directories(output_dir);

    cv::Mat frame;
    while (source.read(frame)) {
        std::string filename = source.getCurrentFilename();
        std::cout << filename << ": " << frame.cols << "x" << frame.rows;

        const nsm::FrameResult& result = pipeline.process(frame);

        float center_offset = nsm::calculate_heading_error(result.center_line, frame.cols);
        std::cout << " -> offset: " << center_offset;
        std::cout << " -> " << result.center_line.points.size() << " points";
        if (result.center_line.valid) {
            std::cout << ", line fitted";
        }
        if (result.corner.detected) {
            std::cout << ", CORNER at (" << static_cast<int>(result.corner.corner_point.x)
                      << "," << static_cast<int>(result.corner.corner_point.y) << ")";
        }
        std::cout << " [" << result.processing_time_ms << " ms]" << std::endl;

        cv::Mat output = nsm::visualize_side_by_side(frame, result);
        fs::path output_path = fs::path(output_dir) / ("processed_" + filename);
        cv::imwrite(output_path.string(), output);

        if (!headless) {
            cv::imshow("NSM Pipeline", output);
            int key = cv::waitKey(100);
            if (key == 'q' || key == 27) break;
        }
    }

    std::cout << "\nProcessed images saved to " << output_dir << std::endl;
    return 0;
}

int runLiveMode(nsm::FrameSource& source, nsm::Pipeline& pipeline) {
    std::cout << "Starting live mode from: " << source.getName() << std::endl;
    std::cout << "Press 'q' or ESC to quit\n" << std::endl;

    cv::Mat frame;
    double fps_smoothed = 0.0;
    int frame_count = 0;

    while (true) {
        if (!source.read(frame)) {
            std::cerr << "Failed to read frame" << std::endl;
            break;
        }

        const nsm::FrameResult& result = pipeline.process(frame);

        float center_offset = nsm::calculate_heading_error(result.center_line, frame.cols);
        std::cout << "Frame " << frame_count << " offset: " << center_offset << std::endl;

        // Smooth FPS calculation
        double fps = 1000.0 / result.processing_time_ms;
        fps_smoothed = (fps_smoothed * 0.9) + (fps * 0.1);

        cv::Mat vis = nsm::visualize_result(frame, result);

        // Draw FPS and info overlay
        std::string info = "FPS: " + std::to_string(static_cast<int>(fps_smoothed));
        if (result.center_line.valid) {
            info += " | Line detected";
        }
        if (result.corner.detected) {
            info += " | CORNER";
        }
        cv::putText(vis, info, cv::Point(10, 30),
                    cv::FONT_HERSHEY_SIMPLEX, 0.7, cv::Scalar(0, 255, 0), 2);

        cv::imshow("NSM Pipeline", vis);

        int key = cv::waitKey(1);
        if (key == 'q' || key == 27) break;

        frame_count++;
        if (frame_count % 100 == 0) {
            std::cout << "Frames: " << frame_count
                      << " | FPS: " << static_cast<int>(fps_smoothed)
                      << " | Processing: " << result.processing_time_ms << " ms" << std::endl;
        }
    }

    return 0;
}

int main(int argc, char** argv) {
    std::string source_path = "../data/stills";
    std::string output_dir = "../data/processed";
    bool headless = false;

    // Parse arguments
    for (int i = 1; i < argc; i++) {
        std::string arg = argv[i];
        if (arg == "--help" || arg == "-h") {
            printUsage(argv[0]);
            return 0;
        } else if (arg == "--headless") {
            headless = true;
        } else if (arg == "--output" && i + 1 < argc) {
            output_dir = argv[++i];
        } else if (arg[0] != '-') {
            source_path = arg;
        }
    }

    std::cout << "OpenCV version: " << CV_VERSION << std::endl;

    // Create pipeline with default config
    nsm::PipelineConfig config;
    nsm::Pipeline pipeline(config);

    // Create frame source
    auto source = nsm::createFrameSource(source_path);
    if (!source) {
        std::cerr << "Error: Could not create frame source from: " << source_path << std::endl;
        return 1;
    }

    if (!source->open()) {
        std::cerr << "Error: Could not open source: " << source_path << std::endl;
        return 1;
    }

    // Determine mode based on source type
    if (auto* img_source = dynamic_cast<nsm::ImageDirectorySource*>(source.get())) {
        return runBatchMode(*img_source, pipeline, output_dir, headless);
    } else {
        if (headless) {
            std::cerr << "Warning: --headless only supported for image directory mode" << std::endl;
        }
        return runLiveMode(*source, pipeline);
    }
}
