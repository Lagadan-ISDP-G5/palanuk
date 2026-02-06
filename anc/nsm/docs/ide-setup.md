# IDE Setup

## Compile Commands

CMake generates a `compile_commands.json` file in the `build/` directory. This file contains the exact compiler invocations for each source file, including:

- Compiler flags
- Include paths (like OpenCV headers at `/opt/homebrew/opt/opencv/include/opencv4/`)
- Macro definitions

Language servers (LSP) like clangd use this file to understand your project structure and provide accurate code intelligence—autocompletion, go-to-definition, and error checking.

Without `compile_commands.json`, the LSP doesn't know where to find external headers like OpenCV, resulting in unresolved includes.

### Generating Compile Commands

The `CMakeLists.txt` includes:

```cmake
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)
```

This tells CMake to generate `build/compile_commands.json` when you run:

```bash
cd build
cmake ..
```

Regenerate this file whenever you add new source files or change compiler flags.

## Zed IDE Setup

Zed uses clangd for C++ language support. To configure clangd to find the compile commands:

### Project Configuration

The `.zed/settings.json` file configures clangd for this project:

```json
{
  "lsp": {
    "clangd": {
      "binary": {
        "arguments": [
          "--compile-commands-dir=build",
          "--background-index"
        ]
      }
    }
  }
}
```

- `--compile-commands-dir=build` tells clangd where to find `compile_commands.json`
- `--background-index` enables background indexing for faster code navigation

### Troubleshooting

If headers still don't resolve:

1. Ensure you've run `cmake ..` in the build directory
2. Restart Zed or use `zed: reload` from the command palette
3. Check that clangd is installed (`brew install llvm` if needed)
4. Verify `build/compile_commands.json` exists and contains entries for your source files
