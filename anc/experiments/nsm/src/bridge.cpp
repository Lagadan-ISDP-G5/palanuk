#include "bridge.h"
#include "line_detection.h"

#include <iox2/node.hpp>
#include <iox2/service_type.hpp>
#include <iox2/service_name.hpp>
#include <iox2/publisher.hpp>
#include <iostream>
#include <optional>

namespace nsm {

namespace {

using namespace iox2;

constexpr auto SERVICE_NAME_HEADING_ERROR = "nsm/heading_error";
constexpr auto SERVICE_NAME_ABS_LINE_GRADIENT = "nsm/abs_line_gradient";
constexpr auto SERVICE_NAME_CORNER_DETECTED = "nsm/corner_detected";
constexpr auto SERVICE_NAME_CORNER_DIRECTION = "nsm/corner_direction";
constexpr auto SERVICE_NAME_CORNER_POINT = "nsm/corner_point";

std::optional<Node<ServiceType::Ipc>> g_node;
std::optional<Publisher<ServiceType::Ipc, HeadingErrorMsg, void>> g_pub_heading_error;
std::optional<Publisher<ServiceType::Ipc, AbsLineGradientMsg, void>> g_pub_abs_line_gradient;
std::optional<Publisher<ServiceType::Ipc, CornerDetectedMsg, void>> g_pub_corner_detected;
std::optional<Publisher<ServiceType::Ipc, CornerDirectionMsg, void>> g_pub_corner_direction;
std::optional<Publisher<ServiceType::Ipc, CornerPointMsg, void>> g_pub_corner_point;

bool g_initialized = false;

}  // namespace

bool init_publishers() {
    if (g_initialized) {
        return true;
    }

    auto node_result = NodeBuilder().create<ServiceType::Ipc>();
    if (!node_result.has_value()) {
        std::cerr << "Failed to create iceoryx2 node" << std::endl;
        return false;
    }
    g_node.emplace(std::move(node_result.value()));

    // Heading error publisher
    {
        auto service_name = ServiceName::create(SERVICE_NAME_HEADING_ERROR);
        if (!service_name.has_value()) {
            std::cerr << "Failed to create service name" << std::endl;
            return false;
        }
        auto service = g_node->service_builder(service_name.value())
            .publish_subscribe<HeadingErrorMsg>()
            .open_or_create();
        if (!service.has_value()) {
            std::cerr << "Failed to create heading_error service" << std::endl;
            return false;
        }
        auto pub = service.value().publisher_builder().create();
        if (!pub.has_value()) {
            std::cerr << "Failed to create heading_error publisher" << std::endl;
            return false;
        }
        g_pub_heading_error.emplace(std::move(pub.value()));
    }

    // Abs line gradient publisher
    {
        auto service_name = ServiceName::create(SERVICE_NAME_ABS_LINE_GRADIENT);
        if (!service_name.has_value()) {
            std::cerr << "Failed to create service name" << std::endl;
            return false;
        }
        auto service = g_node->service_builder(service_name.value())
            .publish_subscribe<AbsLineGradientMsg>()
            .open_or_create();
        if (!service.has_value()) {
            std::cerr << "Failed to create abs_line_gradient service" << std::endl;
            return false;
        }
        auto pub = service.value().publisher_builder().create();
        if (!pub.has_value()) {
            std::cerr << "Failed to create abs_line_gradient publisher" << std::endl;
            return false;
        }
        g_pub_abs_line_gradient.emplace(std::move(pub.value()));
    }

    // Corner detected publisher
    {
        auto service_name = ServiceName::create(SERVICE_NAME_CORNER_DETECTED);
        if (!service_name.has_value()) {
            std::cerr << "Failed to create service name" << std::endl;
            return false;
        }
        auto service = g_node->service_builder(service_name.value())
            .publish_subscribe<CornerDetectedMsg>()
            .open_or_create();
        if (!service.has_value()) {
            std::cerr << "Failed to create corner_detected service" << std::endl;
            return false;
        }
        auto pub = service.value().publisher_builder().create();
        if (!pub.has_value()) {
            std::cerr << "Failed to create corner_detected publisher" << std::endl;
            return false;
        }
        g_pub_corner_detected.emplace(std::move(pub.value()));
    }

    // Corner direction publisher
    {
        auto service_name = ServiceName::create(SERVICE_NAME_CORNER_DIRECTION);
        if (!service_name.has_value()) {
            std::cerr << "Failed to create service name" << std::endl;
            return false;
        }
        auto service = g_node->service_builder(service_name.value())
            .publish_subscribe<CornerDirectionMsg>()
            .open_or_create();
        if (!service.has_value()) {
            std::cerr << "Failed to create corner_direction service" << std::endl;
            return false;
        }
        auto pub = service.value().publisher_builder().create();
        if (!pub.has_value()) {
            std::cerr << "Failed to create corner_direction publisher" << std::endl;
            return false;
        }
        g_pub_corner_direction.emplace(std::move(pub.value()));
    }

    // Corner point publisher
    {
        auto service_name = ServiceName::create(SERVICE_NAME_CORNER_POINT);
        if (!service_name.has_value()) {
            std::cerr << "Failed to create service name" << std::endl;
            return false;
        }
        auto service = g_node->service_builder(service_name.value())
            .publish_subscribe<CornerPointMsg>()
            .open_or_create();
        if (!service.has_value()) {
            std::cerr << "Failed to create corner_point service" << std::endl;
            return false;
        }
        auto pub = service.value().publisher_builder().create();
        if (!pub.has_value()) {
            std::cerr << "Failed to create corner_point publisher" << std::endl;
            return false;
        }
        g_pub_corner_point.emplace(std::move(pub.value()));
    }

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

bool publish_control_vars(const BridgeResult& result) {
    if (!g_initialized) {
        return false;
    }

    bool all_ok = true; // lol, how primitive

    auto heading_result = g_pub_heading_error->send_copy(HeadingErrorMsg{
        .valid = result.heading_error.has_value(),
        .value = result.heading_error.value_or(0.0f)
    });
    if (!heading_result.has_value()) {
        std::cerr << "Failed to publish heading_error" << std::endl;
        all_ok = false;
    }

    auto gradient_result = g_pub_abs_line_gradient->send_copy(AbsLineGradientMsg{
        .valid = result.abs_line_gradient.has_value(),
        .value = result.abs_line_gradient.value_or(0.0f)
    });
    if (!gradient_result.has_value()) {
        std::cerr << "Failed to publish abs_line_gradient" << std::endl;
        all_ok = false;
    }

    auto corner_detected_result = g_pub_corner_detected->send_copy(CornerDetectedMsg{
        .detected = result.corner_detected
    });
    if (!corner_detected_result.has_value()) {
        std::cerr << "Failed to publish corner_detected" << std::endl;
        all_ok = false;
    }

    auto corner_direction_result = g_pub_corner_direction->send_copy(CornerDirectionMsg{
        .x = result.corner_direction.x,
        .y = result.corner_direction.y
    });
    if (!corner_direction_result.has_value()) {
        std::cerr << "Failed to publish corner_direction" << std::endl;
        all_ok = false;
    }

    auto corner_point_result = g_pub_corner_point->send_copy(CornerPointMsg{
        .x = result.corner_point.x,
        .y = result.corner_point.y
    });
    if (!corner_point_result.has_value()) {
        std::cerr << "Failed to publish corner_point" << std::endl;
        all_ok = false;
    }

    return all_ok;
}

}  // namespace nsm
