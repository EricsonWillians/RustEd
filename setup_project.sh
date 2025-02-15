#!/bin/bash
# This script creates a Rust project structure for the Eureka editor.

# Set the project root folder name
PROJECT_ROOT="."

# Create the root directory
mkdir -p "$PROJECT_ROOT"

# Create root files
touch "$PROJECT_ROOT/Cargo.toml"
touch "$PROJECT_ROOT/build.rs"         # Optional build script
touch "$PROJECT_ROOT/README.md"
touch "$PROJECT_ROOT/LICENSE"

# Create resources directories
mkdir -p "$PROJECT_ROOT/resources/common"
mkdir -p "$PROJECT_ROOT/resources/games"
mkdir -p "$PROJECT_ROOT/resources/ports"

# Create src directory and core files
mkdir -p "$PROJECT_ROOT/src"
touch "$PROJECT_ROOT/src/main.rs"
touch "$PROJECT_ROOT/src/lib.rs"

# Create bsp module
mkdir -p "$PROJECT_ROOT/src/bsp"
touch "$PROJECT_ROOT/src/bsp/mod.rs"
touch "$PROJECT_ROOT/src/bsp/bsp_level.rs"
touch "$PROJECT_ROOT/src/bsp/bsp_node.rs"
touch "$PROJECT_ROOT/src/bsp/bsp_util.rs"

# Create document module
mkdir -p "$PROJECT_ROOT/src/document"
touch "$PROJECT_ROOT/src/document/mod.rs"
touch "$PROJECT_ROOT/src/document/document.rs"
touch "$PROJECT_ROOT/src/document/document_module.rs"
touch "$PROJECT_ROOT/src/document/dehconsts.rs"

# Create editor module
mkdir -p "$PROJECT_ROOT/src/editor"
touch "$PROJECT_ROOT/src/editor/mod.rs"
touch "$PROJECT_ROOT/src/editor/commands.rs"
touch "$PROJECT_ROOT/src/editor/cutpaste.rs"
touch "$PROJECT_ROOT/src/editor/generator.rs"
touch "$PROJECT_ROOT/src/editor/hover.rs"
touch "$PROJECT_ROOT/src/editor/linedef.rs"
touch "$PROJECT_ROOT/src/editor/objects.rs"
touch "$PROJECT_ROOT/src/editor/sector.rs"
touch "$PROJECT_ROOT/src/editor/things.rs"
touch "$PROJECT_ROOT/src/editor/vertex.rs"
# Add additional editor modules as needed

# Create ui module
mkdir -p "$PROJECT_ROOT/src/ui"
touch "$PROJECT_ROOT/src/ui/mod.rs"
touch "$PROJECT_ROOT/src/ui/about.rs"
touch "$PROJECT_ROOT/src/ui/browser.rs"
touch "$PROJECT_ROOT/src/ui/canvas.rs"
touch "$PROJECT_ROOT/src/ui/dialog.rs"
touch "$PROJECT_ROOT/src/ui/editor_ui.rs"
touch "$PROJECT_ROOT/src/ui/file.rs"
touch "$PROJECT_ROOT/src/ui/hyper.rs"
touch "$PROJECT_ROOT/src/ui/infobar.rs"
touch "$PROJECT_ROOT/src/ui/linedef_ui.rs"
touch "$PROJECT_ROOT/src/ui/menu.rs"
touch "$PROJECT_ROOT/src/ui/misc.rs"
touch "$PROJECT_ROOT/src/ui/nombre.rs"
touch "$PROJECT_ROOT/src/ui/panelinput.rs"
touch "$PROJECT_ROOT/src/ui/pic.rs"
touch "$PROJECT_ROOT/src/ui/prefs.rs"
touch "$PROJECT_ROOT/src/ui/replace.rs"
touch "$PROJECT_ROOT/src/ui/scroll.rs"
touch "$PROJECT_ROOT/src/ui/sector_ui.rs"
touch "$PROJECT_ROOT/src/ui/sidedef.rs"
touch "$PROJECT_ROOT/src/ui/thing_ui.rs"
touch "$PROJECT_ROOT/src/ui/tile.rs"
touch "$PROJECT_ROOT/src/ui/vertex_ui.rs"
touch "$PROJECT_ROOT/src/ui/window.rs"

# Create utils module
mkdir -p "$PROJECT_ROOT/src/utils"
touch "$PROJECT_ROOT/src/utils/mod.rs"
touch "$PROJECT_ROOT/src/utils/adler.rs"
touch "$PROJECT_ROOT/src/utils/file.rs"
touch "$PROJECT_ROOT/src/utils/tga.rs"
touch "$PROJECT_ROOT/src/utils/util.rs"
# Add more utils as needed

# Create platform module
mkdir -p "$PROJECT_ROOT/src/platform"
touch "$PROJECT_ROOT/src/platform/mod.rs"
touch "$PROJECT_ROOT/src/platform/x11.rs"
touch "$PROJECT_ROOT/src/platform/win.rs"

# Create tests directory and test files
mkdir -p "$PROJECT_ROOT/tests"
touch "$PROJECT_ROOT/tests/mod.rs"
touch "$PROJECT_ROOT/tests/bsp_tests.rs"

echo "Project structure created successfully in '$PROJECT_ROOT'."
