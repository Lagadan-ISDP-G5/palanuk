#include <opencv2/core/types.hpp>
#include <opencv2/highgui.hpp>
#include <opencv2/opencv.hpp>
#include <filesystem>
#include <iostream>
#include <string>
#include <thread>
#include <chrono>

#include "types.h"
#include "pipeline.h"
#include "frame_source.h"
#include "visualization.h"
#include <optional>
#include "bridge.h"

namespace fs = std::filesystem;

void printUsage(const char* program) {
    std::cout << "Usage: " << program << " [OPTIONS] [SOURCE]\n"
              << "\nSOURCE can be:\n"
              << "  <directory>      Process all images in directory\n"
              << "  <video_file>     Process video file\n"
              << "  <camera_id>      Use camera (0, 1, etc.)\n"
              << "  camera:<id>      Use camera explicitly\n"
              << "  iox:<service>    Subscribe to iceoryx2 shared memory frames\n"
              << "\nOPTIONS:\n"
              << "  --headless       Run without display (batch and iceoryx2 modes)\n"
              << "  --output DIR     Output directory for processed images\n"
              << "  --help           Show this help\n"
              << "\nExamples:\n"
              << "  " << program << " ../data/stills\n"
              << "  " << program << " 0\n"
              << "  " << program << " recording.mp4\n"
              << "  " << program << " iox:camera/frames\n";
}

int runBatchMode(nsm::ImageDirectorySource& source, nsm::Pipeline& pipeline,
                 const std::string& output_dir, bool headless) {
    fs::create_directories(output_dir);

    cv::Mat frame;
    nsm::BridgeResult bridge_result;

    while (source.read(frame)) {
        std::string filename = source.getCurrentFilename();
        std::cout << filename << ": " << frame.cols << "x" << frame.rows;

        const nsm::FrameResult& result = pipeline.process(frame);
        nsm::process(result, frame.cols, frame.rows, bridge_result);
        nsm::publish_control_vars(bridge_result);

        if (bridge_result.heading_error.has_value()) {
            std::cout << " -> offset: " << *bridge_result.heading_error;
        }

        std::cout << " -> " << result.center_line.points.size() << " points";
        if (result.center_line.valid) {
            std::cout << ", line fitted";
        }
        if (bridge_result.corner_detected) {
            std::cout << ", CORNER at (" << bridge_result.corner_point.x
                      << "," << bridge_result.corner_point.y << ")";
        }
        std::cout << " [" << result.processing_time_ms << " ms]" << std::endl;

        const cv::Mat& vis_frame = pipeline.getConfig().warp_enabled ? pipeline.getWarped() : frame;
        cv::Mat output = nsm::visualize_side_by_side(vis_frame, result);
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

int runLiveMode(nsm::FrameSource& source, nsm::Pipeline& pipeline, bool headless) {
    std::cout << "Starting live mode from: " << source.getName();
    if (headless) {
        std::cout << " (headless)";
    }
    std::cout << std::endl;
    if (!headless) {
        std::cout << "Press 'q' or ESC to quit\n" << std::endl;
    } else {
        std::cout << "Press Ctrl+C to quit\n" << std::endl;
    }

    cv::Mat frame;
    nsm::BridgeResult bridge_result;
    double fps_smoothed = 0.0;
    int frame_count = 0;

    if (!headless) {
        cv::namedWindow("NSM Pipeline", cv::WINDOW_GUI_NORMAL);
        cv::resizeWindow("NSM Pipeline", 1068, 600);
    }

    while (true) {
        if (!source.read(frame)) {
            // For IPC sources, no frame available is normal - just retry
            if (source.isOpened()) {
                std::this_thread::sleep_for(std::chrono::milliseconds(1));
                continue;
            }
            std::cerr << "Failed to read frame" << std::endl;
            break;
        }

        const nsm::FrameResult& result = pipeline.process(frame);
        nsm::process(result, frame.cols, frame.rows, bridge_result);
        nsm::publish_control_vars(bridge_result);

        if (bridge_result.heading_error.has_value()) {
            std::cout << "Frame " << frame_count << " offset: " << *bridge_result.heading_error << std::endl;
        }

        // Smooth FPS calculation
        double fps = 1000.0 / result.processing_time_ms;
        fps_smoothed = (fps_smoothed * 0.9) + (fps * 0.1);

        if (!headless) {
            const cv::Mat& vis_frame = pipeline.getConfig().warp_enabled ? pipeline.getWarped() : frame;
            cv::Mat vis = nsm::visualize_result(vis_frame, result);

            // Draw FPS and info overlay
            std::string info = "FPS: " + std::to_string(static_cast<int>(fps_smoothed));
            if (result.center_line.valid) {
                info += " | Line detected";
            }
            if (bridge_result.corner_detected) {
                info += " | CORNER";
            }
            if (bridge_result.heading_error.has_value()) {
                std::string heading_error = "heading_err: " + std::to_string(bridge_result.heading_error.value());
                cv::putText(vis, heading_error, cv::Point(10, 50), cv::FONT_HERSHEY_SIMPLEX, 1.2, cv::Scalar(0, 255, 0), 2);
            }

            cv::putText(vis, info, cv::Point(10, 30),
                        cv::FONT_HERSHEY_SIMPLEX, 0.7, cv::Scalar(0, 255, 0), 2);

            cv::imshow("NSM Pipeline", vis);

            int key = cv::waitKey(1);
            if (key == 'q' || key == 27) break;
        }

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

    // Initialize iceoryx2 publishers
    if (!nsm::init_publishers()) {
        std::cerr << "Warning: Failed to initialize iceoryx2 publishers" << std::endl;
    }

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
    int result;
    if (auto* img_source = dynamic_cast<nsm::ImageDirectorySource*>(source.get())) {
        result = runBatchMode(*img_source, pipeline, output_dir, headless);
    } else {
        bool is_iox_source = dynamic_cast<nsm::IceoryxSource*>(source.get()) != nullptr;
        if (headless && !is_iox_source) {
            std::cerr << "Warning: --headless only supported for image directory and iceoryx2 modes" << std::endl;
            headless = false;
        }
        result = runLiveMode(*source, pipeline, headless);
    }

    nsm::shutdown_publishers();
    return result;
}
