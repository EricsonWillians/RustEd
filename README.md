# RustEd

RustEd is a high-performance, concurrent Doom map editor with procedural generation, inspired by the classic Eureka editor. Written entirely in Rust, RustEd leverages modern programming practices and a modular architecture to provide a safe, extensible, and responsive editing experience for game level designers.

---

## Table of Contents

- [Overview](#overview)
- [Features](#features)
- [Architecture & Project Structure](#architecture--project-structure)
- [Getting Started](#getting-started)
  - [Prerequisites](#prerequisites)
  - [Installation](#installation)
  - [Building the Project](#building-the-project)
- [Usage](#usage)
  - [Running the Editor](#running-the-editor)
  - [Testing](#testing)
- [Modules Overview](#modules-overview)
- [Contributing](#contributing)
- [License](#license)
- [Acknowledgments](#acknowledgments)
- [Contact](#contact)

---

## Overview

RustEd is designed to be:

- **High-performance:** Leveraging Rust's zero-cost abstractions and concurrency capabilities, RustEd can handle complex level data and procedural generation without sacrificing responsiveness.
- **Concurrent:** Built with modern Rust libraries like [Rayon](https://crates.io/crates/rayon) for parallelism and [eframe/egui](https://crates.io/crates/eframe) for an immediate-mode GUI, RustEd supports smooth, non-blocking editing even on large maps.
- **Safe and Reliable:** With Rust’s strict compile-time safety guarantees, RustEd minimizes common runtime errors and memory bugs.
- **Modular & Extensible:** The project is organized into clear modules (e.g., BSP processing, document management, editor logic, UI, etc.), making it easy to extend and maintain.

---

## Features

- **Procedural Generation:** Generate maps using cutting-edge algorithms.
- **Parallel Processing:** Utilize Rayon for concurrent data processing.
- **Modern UI:** An immediate-mode interface built using eframe/egui, featuring:
  - Top menu bar
  - Side panels for tool selection
  - Central canvas for map display and editing
  - Bottom status bar for feedback
- **Robust Level Format Support:** Safely import, modify, and export level data.
- **Extensible Architecture:** Clearly separated modules make it easy to add new features.

---

## Architecture & Project Structure

The project is organized into several core directories, each responsible for a distinct aspect of RustEd’s functionality:

```
RustEd/
├── Cargo.toml                # Project configuration and dependencies
├── build.rs                  # Build script for resource embedding (optional)
├── LICENSE                   # Project license
├── README.md                 # Project documentation (this file)
├── resources/                # Assets, configuration files, etc.
│   ├── common/
│   ├── games/
│   └── ports/
├── setup_project.sh          # Script to set up the project structure
├── src/
│   ├── bsp/                  # Level geometry, BSP, and blockmap generation
│   │   ├── bsp_level.rs
│   │   ├── bsp_node.rs
│   │   ├── bsp_util.rs
│   │   └── mod.rs
│   ├── document/             # Level data structures and file parsing
│   │   ├── dehconsts.rs
│   │   ├── document_module.rs
│   │   ├── document.rs
│   │   └── mod.rs
│   ├── editor/               # Core editor logic and commands
│   │   ├── commands.rs
│   │   ├── cutpaste.rs
│   │   ├── generator.rs
│   │   ├── hover.rs
│   │   ├── linedef.rs
│   │   ├── mod.rs
│   │   ├── objects.rs
│   │   ├── sector.rs
│   │   ├── things.rs
│   │   └── vertex.rs
│   ├── lib.rs                # Library entry point; re-exports modules
│   ├── main.rs               # Main application entry point (UI/egui integration)
│   ├── platform/             # Platform-specific code (e.g., Windows, X11)
│   │   ├── mod.rs
│   │   ├── win.rs
│   │   └── x11.rs
│   ├── ui/                   # User interface code (menus, dialogs, panels)
│   │   ├── about.rs
│   │   ├── browser.rs
│   │   ├── canvas.rs
│   │   ├── dialog.rs
│   │   ├── editor_ui.rs
│   │   ├── file.rs
│   │   ├── hyper.rs
│   │   ├── infobar.rs
│   │   ├── linedef_ui.rs
│   │   ├── menu.rs
│   │   ├── misc.rs
│   │   ├── mod.rs
│   │   ├── nombre.rs
│   │   ├── panelinput.rs
│   │   ├── pic.rs
│   │   ├── prefs.rs
│   │   ├── replace.rs
│   │   ├── scroll.rs
│   │   ├── sector_ui.rs
│   │   ├── sidedef.rs
│   │   ├── thing_ui.rs
│   │   ├── tile.rs
│   │   ├── vertex_ui.rs
│   │   └── main_window.rs   # Main UI module using egui/eframe
│   └── utils/                # Utility functions and helper modules
│       ├── adler.rs
│       ├── file.rs
│       ├── mod.rs
│       ├── tga.rs
│       └── util.rs
└── tests/                    # Integration and unit tests for the project
    ├── bsp_tests.rs
    └── mod.rs
```

Each module is self-contained, ensuring that you can test and develop components independently.

---

## Getting Started

### Prerequisites

- **Rust & Cargo:** Install the latest stable Rust toolchain from [rust-lang.org](https://www.rust-lang.org/tools/install).
- **Git:** Required for cloning the repository.
- **Additional Libraries:** See [Cargo.toml](Cargo.toml) for the full list of dependencies.

### Installation

1. **Clone the Repository:**

   ```bash
   git clone https://github.com/EricsonWillians/RustEd.git
   cd RustEd
   ```

2. **(Optional) Run Setup Script:**

   If provided, run the setup script to create the project structure:

   ```bash
   chmod +x setup_project.sh
   ./setup_project.sh
   ```

### Building the Project

Build the project using Cargo:

```bash
cargo build --release
```

For development (with faster compile times):

```bash
cargo build
```

---

## Usage

### Running the Editor

After building, run the application with:

```bash
cargo run
```

This launches the RustEd editor window powered by **eframe/egui**. The main window displays:
- A top menu bar with options (File, Edit, View, Help).
- A left panel with tool buttons.
- A central canvas area (placeholder for map editing).
- A bottom status bar showing messages.

### Testing

RustEd uses Cargo’s built-in testing framework. To run all tests:

```bash
cargo test
```

You can also run tests for specific modules. For example, to test the document module:

```bash
cargo test --lib document::document
```

Tests are organized in each module as well as in the `/tests` directory for integration testing.

---

## Modules Overview

### bsp
Handles level geometry, BSP tree generation, and blockmap processing.  
Key files:
- **bsp_level.rs:** Functions for processing level nodes.
- **bsp_node.rs:** Node structure and algorithms.
- **bsp_util.rs:** Utility functions for BSP generation.

### document
Manages level file formats and data structures such as vertices, linedefs, sectors, sidedefs, and things.  
Key files:
- **document.rs:** Core document structure and methods.
- **dehconsts.rs:** Definitions of constants and macros for level data.

### editor
Contains core editing functionality:
- **commands.rs:** Editor commands and keybindings.
- **generator.rs:** Procedural generation algorithms.
- **hover.rs:** Mouse hover and selection logic.
- **linedef.rs, sector.rs, vertex.rs, etc.:** Object-specific editing routines.

### platform
Holds platform-specific code for Windows, Linux (X11), etc.  
Key files:
- **win.rs:** Windows-specific initialization.
- **x11.rs:** Linux/X11-specific functions.

### ui
Provides the graphical user interface:
- **main_window.rs:** Main UI built with eframe/egui.
- Other files define dialogs, panels, and interactive widgets.

### utils
Miscellaneous helper functions (file handling, image processing, checksums, etc.).

---

## Contributing

Contributions are welcome! Please follow these guidelines:
- **Fork and Clone:** Create your own fork and clone it locally.
- **Branching:** Use feature branches for your changes.
- **Tests:** Write tests for new features or bug fixes.
- **Pull Requests:** Submit a pull request with a detailed description of your changes.
- **Coding Standards:** Follow Rust’s idiomatic practices and the project's coding style.

For major changes, please open an issue first to discuss your plans.

---

## License

RustEd is licensed under the [GNU LICENSE](LICENSE). See the LICENSE file for details.

---

## Acknowledgments

- **Eureka Editor:** Inspired by the classic Eureka DOOM Editor.
- **Rust Community:** Thanks to the Rust community for the tools, libraries, and resources.
- **Open-Source Contributors:** Thanks to the contributors of dependencies like eframe, egui, Rayon, and others.

---

## Contact

For questions, suggestions, or contributions, please reach out to:

- **Ericson Willians** – [ericsonwillians@protonmail.com](mailto:ericsonwillians@protonmail.com)
- GitHub: [EricsonWillians/RustEd](https://github.com/EricsonWillians/RustEd)

---