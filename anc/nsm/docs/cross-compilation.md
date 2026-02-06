# Cross-Compilation for Raspberry Pi

This document describes how to cross-compile `anc_nsm` for Raspberry Pi (aarch64) from a macOS or Linux host.

## Overview

The binary is cross-compiled on your development machine but **links dynamically against OpenCV** on the Pi. This means:
- You need the cross-compiler toolchain on your host
- You need OpenCV headers/libs accessible during compilation (from a Pi sysroot)
- The Pi must have OpenCV installed at runtime

## Prerequisites

### On macOS (via Homebrew)

```bash
# Install cross-compiler
brew install aarch64-elf-gcc
# Or use a Linux cross-toolchain via Docker (recommended)
```

### On Linux (Ubuntu/Debian)

```bash
sudo apt install gcc-aarch64-linux-gnu g++-aarch64-linux-gnu
```

### On the Raspberry Pi

```bash
# Install OpenCV (will be linked at runtime)
sudo apt update
sudo apt install libopencv-dev
```

## Setting Up the Sysroot

Cross-compilation needs access to the Pi's headers and libraries. There are two approaches:

### Option A: Rsync from Pi (Recommended)

```bash
# Create sysroot directory
mkdir -p ~/pi-sysroot

# Sync required directories from Pi
PI_HOST=pi@raspberrypi.local
rsync -e "ssh -i ~/gipop_plc" -avzL --rsync-path="sudo rsync" $PI_HOST:/usr/include ~/pi-sysroot/usr/
rsync -e "ssh -i ~/gipop_plc" -avzL --rsync-path="sudo rsync" $PI_HOST:/usr/lib/aarch64-linux-gnu ~/pi-sysroot/usr/lib/
rsync -e "ssh -i ~/gipop_plc" -avzL --rsync-path="sudo rsync" $PI_HOST:/lib/aarch64-linux-gnu ~/pi-sysroot/lib/
rsync -e "ssh -i ~/gipop_plc" -avzL --rsync-path="sudo rsync" $PI_HOST:/usr/lib/gcc/ ~/pi-sysroot/usr/lib/gcc/

## Configuring the Toolchain

Edit `cmake/toolchain-aarch64-linux.cmake`:

```cmake
# Point to your sysroot
set(CMAKE_SYSROOT /Users/yourname/pi-sysroot)

# OpenCV cmake config location (inside sysroot)
set(OpenCV_DIR ${CMAKE_SYSROOT}/usr/lib/aarch64-linux-gnu/cmake/opencv4)
```

## Building

### Using CMake Presets (Recommended)

```bash
# List available presets
cmake --list-presets

# Configure and build for Pi
cmake --preset pi
cmake --build --preset pi

# Build for host (native)
cmake --preset host
cmake --build --preset host
```

For future host builds: Override the deps path:
  cmake -DIOX2_PREFIX_PATH=/path/to/host/iceoryx2 ..

## Runtime Dependencies

On the Pi, ensure OpenCV is installed:

```bash
# Check OpenCV is available
pkg-config --modversion opencv4

# If missing
sudo apt install libopencv-dev
```

The binary links against these shared libraries (verify with `ldd`):

```bash
ldd ./anc_nsm
# Should show libopencv_core.so, libopencv_imgproc.so, etc.
```

## Troubleshooting

### "cannot find -lopencv_core"

OpenCV not found in sysroot. Verify:
```bash
ls $SYSROOT/usr/lib/aarch64-linux-gnu/libopencv_core.so
```

### "GLIBC_X.XX not found" at runtime

Your cross-compiler's glibc is newer than the Pi's. Solutions:
1. Use a toolchain matching the Pi's Debian version
2. Update the Pi: `sudo apt upgrade`
3. Build with an older toolchain (e.g., via Docker with matching Debian)

### Camera not working on Pi

```bash
# Check camera is detected
libcamera-hello --list-cameras

# For OpenCV, you may need V4L2
sudo modprobe bcm2835-v4l2
```
