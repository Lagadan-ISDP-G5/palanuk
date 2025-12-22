# Cross-compilation toolchain for Raspberry Pi (aarch64) using Clang
set(CMAKE_SYSTEM_NAME Linux)
set(CMAKE_SYSTEM_PROCESSOR aarch64)

# Use Clang as cross-compiler
set(CMAKE_C_COMPILER clang)
set(CMAKE_CXX_COMPILER clang++)

# Target triple for aarch64 Linux
set(CMAKE_C_COMPILER_TARGET aarch64-linux-gnu)
set(CMAKE_CXX_COMPILER_TARGET aarch64-linux-gnu)

# Use lld linker (supports cross-linking)
set(CMAKE_EXE_LINKER_FLAGS "-fuse-ld=lld")
set(CMAKE_SHARED_LINKER_FLAGS "-fuse-ld=lld")

# Sysroot - set this to your Pi's sysroot or mounted filesystem
# Must contain /usr/lib/gcc/aarch64-linux-gnu/<version>/ from the Pi
set(CMAKE_SYSROOT /Users/ander/pi-sysroot)

# Add GCC library path (adjust version as needed)
set(GCC_VERSION 12)
set(CMAKE_EXE_LINKER_FLAGS "${CMAKE_EXE_LINKER_FLAGS} -L${CMAKE_SYSROOT}/usr/lib/gcc/aarch64-linux-gnu/${GCC_VERSION}")

# Where to look for libraries/headers on target
set(CMAKE_FIND_ROOT_PATH_MODE_PROGRAM NEVER)
set(CMAKE_FIND_ROOT_PATH_MODE_LIBRARY ONLY)
set(CMAKE_FIND_ROOT_PATH_MODE_INCLUDE ONLY)
set(CMAKE_FIND_ROOT_PATH_MODE_PACKAGE ONLY)

# OpenCV path on Pi (when using rsync'd sysroot)
set(OpenCV_DIR ${CMAKE_SYSROOT}/lib/aarch64-linux-gnu/cmake/opencv4)
