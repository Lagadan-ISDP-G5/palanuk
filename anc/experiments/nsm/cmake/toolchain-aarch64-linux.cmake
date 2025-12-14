# Cross-compilation toolchain for Raspberry Pi (aarch64)
set(CMAKE_SYSTEM_NAME Linux)
set(CMAKE_SYSTEM_PROCESSOR aarch64)

# Cross compiler - adjust path if using a different toolchain
set(CMAKE_C_COMPILER aarch64-linux-gnu-gcc)
set(CMAKE_CXX_COMPILER aarch64-linux-gnu-g++)

# Sysroot - set this to your Pi's sysroot or mounted filesystem
# set(CMAKE_SYSROOT /path/to/pi/sysroot)

# Where to look for libraries/headers on target
set(CMAKE_FIND_ROOT_PATH_MODE_PROGRAM NEVER)
set(CMAKE_FIND_ROOT_PATH_MODE_LIBRARY ONLY)
set(CMAKE_FIND_ROOT_PATH_MODE_INCLUDE ONLY)
set(CMAKE_FIND_ROOT_PATH_MODE_PACKAGE ONLY)

# OpenCV path on Pi (when using rsync'd sysroot)
# set(OpenCV_DIR /path/to/pi/sysroot/usr/lib/aarch64-linux-gnu/cmake/opencv4)
