cmake_minimum_required(VERSION 3.10)
project(anc_nsm)

set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

find_package(OpenCV REQUIRED)

include_directories(${OpenCV_INCLUDE_DIRS})
include_directories(${CMAKE_SOURCE_DIR}/src)

set(NSM_SOURCES
    src/main.cpp
    src/preprocessing.cpp
    src/line_detection.cpp
    src/corner_detection.cpp
    src/visualization.cpp
    src/frame_source.cpp
    src/pipeline.cpp
    src/bridge.cpp
)

set(NSM_HEADERS
    src/types.h
    src/preprocessing.h
    src/line_detection.h
    src/corner_detection.h
    src/visualization.h
    src/frame_source.h
    src/pipeline.h
    src/bridge.h
)

add_executable(${PROJECT_NAME} ${NSM_SOURCES} ${NSM_HEADERS})

target_link_libraries(${PROJECT_NAME} ${OpenCV_LIBS})

# Optional: build as library for integration
option(BUILD_NSM_LIB "Build nsm as a static library" OFF)
if(BUILD_NSM_LIB)
    set(NSM_LIB_SOURCES
        src/preprocessing.cpp
        src/line_detection.cpp
        src/corner_detection.cpp
        src/visualization.cpp
        src/frame_source.cpp
        src/pipeline.cpp
        src/bridge.cpp
    )
    add_library(nsm_lib STATIC ${NSM_LIB_SOURCES})
    target_link_libraries(nsm_lib ${OpenCV_LIBS})
endif()
