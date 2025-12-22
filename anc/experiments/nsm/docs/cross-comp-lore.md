# Cross-Compilation Lore: macOS to Raspberry Pi (aarch64)

This document chronicles the issues encountered while setting up cross-compilation from macOS (Apple Silicon) to Raspberry Pi (aarch64 Linux) and how they were resolved.

## The Setup

- **Host**: macOS (Apple Silicon / arm64)
- **Target**: Raspberry Pi (aarch64 Linux)
- **Toolchain**: Clang + lld (LLVM)
- **Sysroot**: `~/pi-sysroot` (rsync'd from the Pi)

---

## Issue 1: Wrong Cross-Compiler (aarch64-elf-gcc)

### Symptom
```
/opt/homebrew/opt/aarch64-elf-binutils/bin/aarch64-elf-ld: cannot find crt0.o: No such file or directory
/opt/homebrew/opt/aarch64-elf-binutils/bin/aarch64-elf-ld: cannot find -lc: No such file or directory
```

### Cause
The original toolchain used `aarch64-elf-gcc` which is a **bare-metal** cross-compiler (for targets without an OS). It looks for `crt0.o` (bare-metal startup) and expects newlib.

For Linux targets, you need a **Linux cross-compiler** that uses `crt1.o` and glibc.

| Toolchain | Startup | C Library | Target |
|-----------|---------|-----------|--------|
| `aarch64-elf-gcc` | `crt0.o` | newlib | bare-metal |
| `aarch64-linux-gnu-gcc` | `crt1.o` | glibc | Linux |

### Fix
Use **Clang** instead. Clang is a native cross-compiler - it can target any supported architecture with just the `--target` flag and a sysroot.

```cmake
set(CMAKE_C_COMPILER clang)
set(CMAKE_CXX_COMPILER clang++)
set(CMAKE_C_COMPILER_TARGET aarch64-linux-gnu)
set(CMAKE_CXX_COMPILER_TARGET aarch64-linux-gnu)
```

---

## Issue 2: macOS Linker Doesn't Understand Linux Flags

### Symptom
```
ld: unknown options: --sysroot=/Users/ander/pi-sysroot -EL --hash-style=gnu --eh-frame-hdr -dynamic-linker --as-needed
```

### Cause
Clang was invoking the macOS `ld` linker, which doesn't understand Linux linker flags like `--hash-style=gnu`.

### Fix
Install and use **lld** (LLVM's linker) which supports cross-linking:

```bash
brew install lld
```

```cmake
set(CMAKE_EXE_LINKER_FLAGS "-fuse-ld=lld")
set(CMAKE_SHARED_LINKER_FLAGS "-fuse-ld=lld")
```

---

## Issue 3: Missing GCC Runtime Files

### Symptom
```
ld.lld: error: cannot open crtbeginS.o: No such file or directory
ld.lld: error: unable to find library -lgcc
ld.lld: error: unable to find library -lgcc_s
ld.lld: error: cannot open crtendS.o: No such file or directory
```

### Cause
The sysroot was missing the GCC support files (`crtbeginS.o`, `crtendS.o`, `libgcc.a`). These live in `/usr/lib/gcc/aarch64-linux-gnu/<version>/` on the Pi.

### Fix
Rsync the GCC directory from the Pi:

```bash
rsync -avz pi@<pi-ip>:/usr/lib/gcc /Users/ander/pi-sysroot/usr/lib/
```

Add the GCC library path to the toolchain:

```cmake
set(GCC_VERSION 12)
set(CMAKE_EXE_LINKER_FLAGS "${CMAKE_EXE_LINKER_FLAGS} -L${CMAKE_SYSROOT}/usr/lib/gcc/aarch64-linux-gnu/${GCC_VERSION}")
```

---

## Issue 4: Missing Dynamic Linker Symlink

### Symptom
```
ld.lld: error: /Users/ander/pi-sysroot/lib/aarch64-linux-gnu/libc.so:5: cannot find /lib/ld-linux-aarch64.so.1 inside /Users/ander/pi-sysroot
```

### Cause
The `libc.so` linker script references `/lib/ld-linux-aarch64.so.1` but the sysroot has it at `/lib/aarch64-linux-gnu/ld-linux-aarch64.so.1`.

### Fix
Create a symlink:

```bash
ln -sf aarch64-linux-gnu/ld-linux-aarch64.so.1 ~/pi-sysroot/lib/ld-linux-aarch64.so.1
```

---

## Issue 5: Missing OpenCV Include Directory

### Symptom
```
CMake Warning: OpenCV: Include directory doesn't exist: '/Users/ander/pi-sysroot/include/opencv4'
```

Followed by:
```
fatal error: 'opencv2/opencv.hpp' file not found
```

### Cause
OpenCV's CMake config expected includes at `${sysroot}/include/opencv4` but they were at `${sysroot}/usr/include/opencv4`.

### Fix
Create a symlink:

```bash
mkdir -p ~/pi-sysroot/include
ln -sf ../usr/include/opencv4 ~/pi-sysroot/include/opencv4
```

---

## Issue 6: Broken Symlinks from Symlink "Fix" Script

### Symptom
```
fatal error: 'asm/errno.h' file not found
```

### Cause
A script intended to fix absolute symlinks in the sysroot had buggy sed logic:

```bash
# BROKEN - DO NOT USE
find . -type l | while read link; do
    target=$(readlink "$link")
    if [[ "$target" == /* ]]; then
        ln -sf "$(echo "$link" | sed 's|[^/]*/|../|g' | sed 's|/[^/]*$||')$target" "$link"
    fi
done
```

This created malformed relative paths that pointed to non-existent locations.

### Fix
Re-rsync the includes with `-L` to copy actual files instead of symlinks:

```bash
rm -rf ~/pi-sysroot/usr/include
rsync -avzL pi@<pi-ip>:/usr/include ~/pi-sysroot/usr/
```

The `-L` flag tells rsync to follow symlinks and copy the target files.

---

## Issue 7: Missing `asm` Headers

### Symptom
```
fatal error: 'asm/errno.h' file not found
```

### Cause
The `asm` directory was missing from the sysroot includes. On aarch64, most `asm/*.h` headers are identical to `asm-generic/*.h`.

### Fix
Create a symlink:

```bash
ln -sf asm-generic ~/pi-sysroot/usr/include/asm
```

---

## Issue 8: Incorrect Include Paths in CMakeLists.txt

### Symptom
```
error: use of undeclared identifier 'errno'
#define errno errno
```

### Cause
Adding overly specific include paths like `/usr/include/asm` to `include_directories()` broke the standard header include order, causing `errno.h` to not properly define `errno`.

### Fix
Remove the manual include paths - the sysroot handles this automatically:

```cmake
# DON'T DO THIS:
# include_directories(/Users/ander/pi-sysroot/usr/include)
# include_directories(/Users/ander/pi-sysroot/usr/include/asm)

# Just use the standard OpenCV includes:
include_directories(${OpenCV_INCLUDE_DIRS})
```

---

## Final Working Toolchain File

`cmake/toolchain-aarch64-linux.cmake`:

```cmake
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
```

---

## Sysroot Setup Checklist

1. **Basic sysroot rsync** (with `-L` to resolve symlinks):
   ```bash
   rsync -avzL pi@<pi-ip>:/usr/include ~/pi-sysroot/usr/
   rsync -avzL pi@<pi-ip>:/usr/lib ~/pi-sysroot/usr/
   rsync -avzL pi@<pi-ip>:/lib ~/pi-sysroot/
   ```

2. **GCC support files**:
   ```bash
   rsync -avz pi@<pi-ip>:/usr/lib/gcc ~/pi-sysroot/usr/lib/
   ```

3. **Create required symlinks**:
   ```bash
   # Dynamic linker
   ln -sf aarch64-linux-gnu/ld-linux-aarch64.so.1 ~/pi-sysroot/lib/ld-linux-aarch64.so.1

   # OpenCV includes
   mkdir -p ~/pi-sysroot/include
   ln -sf ../usr/include/opencv4 ~/pi-sysroot/include/opencv4

   # asm headers (if missing)
   ln -sf asm-generic ~/pi-sysroot/usr/include/asm
   ```

4. **Install lld**:
   ```bash
   brew install lld
   ```

---

## Build Commands

```bash
# Configure
cmake --preset pi

# Build
cmake --build --preset pi

# Verify
file build-pi/anc_nsm
# Should output: ELF 64-bit LSB pie executable, ARM aarch64, ...
```
