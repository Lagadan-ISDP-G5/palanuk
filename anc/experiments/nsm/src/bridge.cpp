#include "bridge.h"
#include "line_detection.h"

#include <iox2/node.hpp>
#include <iox2/service_type.hpp>
#include <iox2/publisher.hpp>
#include <iostream>
#include <memory>

namespace nsm {

namespace {

using namespace iox2;

constexpr auto SERVICE_NAME_HEADING_ERROR = "nsm/heading_error";
constexpr auto SERVICE_NAME_ABS_LINE_GRADIENT = "nsm/abs_line_gradient";
constexpr auto SERVICE_NAME_CORNER_DETECTED = "nsm/corner_detected";
constexpr auto SERVICE_NAME_CORNER_DIRECTION = "nsm/corner_direction";
constexpr auto SERVICE_NAME_CORNER_POINT = "nsm/corner_point";

std::unique_ptr<Node<ServiceType::Ipc>> g_node;
std::unique_ptr<Publisher<ServiceType::Ipc, HeadingErrorMsg, void>> g_pub_heading_error;
std::unique_ptr<Publisher<ServiceType::Ipc, AbsLineGradientMsg, void>> g_pub_abs_line_gradient;
std::unique_ptr<Publisher<ServiceType::Ipc, CornerDetectedMsg, void>> g_pub_corner_detected;
std::unique_ptr<Publisher<ServiceType::Ipc, CornerDirectionMsg, void>> g_pub_corner_direction;
std::unique_ptr<Publisher<ServiceType::Ipc, CornerPointMsg, void>> g_pub_corner_point;

bool g_initialized = false;

}  // namespace

bool init_publishers() {
    if (g_initialized) {
        return true;
    }

    auto node_result = NodeBuilder().create<ServiceType::Ipc>();
    if (node_result.has_error()) {
        std::cerr << "Failed to create iceoryx2 node" << std::endl;
        return false;
    }
    g_node = std::make_unique<Node<ServiceType::Ipc>>(std::move(node_result.value()));

    // Heading error publisher
    auto heading_error_service = g_node->service_builder(ServiceName::create(SERVICE_NAME_HEADING_ERROR).value())
        .publish_subscribe<HeadingErrorMsg>()
        .open_or_create();
    if (heading_error_service.has_error()) {
        std::cerr << "Failed to create heading_error service" << std::endl;
        return false;
    }
    auto heading_error_pub = heading_error_service.value().publisher_builder().create();
    if (heading_error_pub.has_error()) {
        std::cerr << "Failed to create heading_error publisher" << std::endl;
        return false;
    }
    g_pub_heading_error = std::make_unique<Publisher<ServiceType::Ipc, HeadingErrorMsg, void>>(
        std::move(heading_error_pub.value()));

    // Abs line gradient publisher
    auto abs_line_gradient_service = g_node->service_builder(ServiceName::create(SERVICE_NAME_ABS_LINE_GRADIENT).value())
        .publish_subscribe<AbsLineGradientMsg>()
        .open_or_create();
    if (abs_line_gradient_service.has_error()) {
        std::cerr << "Failed to create abs_line_gradient service" << std::endl;
        return false;
    }
    auto abs_line_gradient_pub = abs_line_gradient_service.value().publisher_builder().create();
    if (abs_line_gradient_pub.has_error()) {
        std::cerr << "Failed to create abs_line_gradient publisher" << std::endl;
        return false;
    }
    g_pub_abs_line_gradient = std::make_unique<Publisher<ServiceType::Ipc, AbsLineGradientMsg, void>>(
        std::move(abs_line_gradient_pub.value()));

    // Corner detected publisher
    auto corner_detected_service = g_node->service_builder(ServiceName::create(SERVICE_NAME_CORNER_DETECTED).value())
        .publish_subscribe<CornerDetectedMsg>()
        .open_or_create();
    if (corner_detected_service.has_error()) {
        std::cerr << "Failed to create corner_detected service" << std::endl;
        return false;
    }
    auto corner_detected_pub = corner_detected_service.value().publisher_builder().create();
    if (corner_detected_pub.has_error()) {
        std::cerr << "Failed to create corner_detected publisher" << std::endl;
        return false;
    }
    g_pub_corner_detected = std::make_unique<Publisher<ServiceType::Ipc, CornerDetectedMsg, void>>(
        std::move(corner_detected_pub.value()));

    // Corner direction publisher
    auto corner_direction_service = g_node->service_builder(ServiceName::create(SERVICE_NAME_CORNER_DIRECTION).value())
        .publish_subscribe<CornerDirectionMsg>()
        .open_or_create();
    if (corner_direction_service.has_error()) {
        std::cerr << "Failed to create corner_direction service" << std::endl;
        return false;
    }
    auto corner_direction_pub = corner_direction_service.value().publisher_builder().create();
    if (corner_direction_pub.has_error()) {
        std::cerr << "Failed to create corner_direction publisher" << std::endl;
        return false;
    }
    g_pub_corner_direction = std::make_unique<Publisher<ServiceType::Ipc, CornerDirectionMsg, void>>(
        std::move(corner_direction_pub.value()));

    // Corner point publisher
    auto corner_point_service = g_node->service_builder(ServiceName::create(SERVICE_NAME_CORNER_POINT).value())
        .publish_subscribe<CornerPointMsg>()
        .open_or_create();
    if (corner_point_service.has_error()) {
        std::cerr << "Failed to create corner_point service" << std::endl;
        return false;
    }
    auto corner_point_pub = corner_point_service.value().publisher_builder().create();
    if (corner_point_pub.has_error()) {
        std::cerr << "Failed to create corner_point publisher" << std::endl;
        return false;
    }
    g_pub_corner_point = std::make_unique<Publisher<ServiceType::Ipc, CornerPointMsg, void>>(
        std::move(corner_point_pub.value()));

    g_initialized = true;
    std::cout << "iceoryx2 publishers initialized" << std::endl;
    return true;
}

void shutdown_publishers() {
    g_pub_heading_error.reset();
    g_pub_abs_line_gradient.reset();
    g_pub_corner_detected.reset();
    g_pub_corner_direction.reset();
    g_pub_corner_point.reset();
    g_node.reset();
    g_initialized = false;
}

void process(const FrameResult& frame_result, int frame_width, BridgeResult& out) {
    out.reset();

    out.heading_error = calculate_heading_error(frame_result.center_line, frame_width);
    out.abs_line_gradient = calculate_abs_line_gradient(frame_result.center_line);
    out.corner_detected = frame_result.corner.detected;
    out.corner_direction = frame_result.corner.horizontal_direction;
    out.corner_point = frame_result.corner.corner_point;
}

void publish(const BridgeResult& result) {
    if (!g_initialized) {
        return;
    }

    // Publish heading error
    auto heading_sample = g_pub_heading_error->loan_uninit();
    if (heading_sample.has_value()) {
        auto& payload = heading_sample.value().write_payload(HeadingErrorMsg{
            .valid = result.heading_error.has_value(),
            .value = result.heading_error.value_or(0.0f)
        });
        send(std::move(heading_sample).value());
    }

    // Publish abs line gradient
    auto gradient_sample = g_pub_abs_line_gradient->loan_uninit();
    if (gradient_sample.has_value()) {
        auto& payload = gradient_sample.value().write_payload(AbsLineGradientMsg{
            .valid = result.abs_line_gradient.has_value(),
            .value = result.abs_line_gradient.value_or(0.0f)
        });
        send(std::move(gradient_sample).value());
    }

    // Publish corner detected
    auto corner_detected_sample = g_pub_corner_detected->loan_uninit();
    if (corner_detected_sample.has_value()) {
        auto& payload = corner_detected_sample.value().write_payload(CornerDetectedMsg{
            .detected = result.corner_detected
        });
        send(std::move(corner_detected_sample).value());
    }

    // Publish corner direction
    auto corner_direction_sample = g_pub_corner_direction->loan_uninit();
    if (corner_direction_sample.has_value()) {
        auto& payload = corner_direction_sample.value().write_payload(CornerDirectionMsg{
            .x = result.corner_direction.x,
            .y = result.corner_direction.y
        });
        send(std::move(corner_direction_sample).value());
    }

    // Publish corner point
    auto corner_point_sample = g_pub_corner_point->loan_uninit();
    if (corner_point_sample.has_value()) {
        auto& payload = corner_point_sample.value().write_payload(CornerPointMsg{
            .x = result.corner_point.x,
            .y = result.corner_point.y
        });
        send(std::move(corner_point_sample).value());
    }
}

}  // namespace nsm
