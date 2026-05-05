# GCode Web Editor Example

This example demonstrates how to use the `gcode_editor` library in a web browser using egui's web backend (eframe).

## Features

- Full GCode syntax highlighting in the browser
- Line numbers with active line highlighting
- Copy to clipboard functionality
- Real-time statistics (line count, command counts)
- Dark theme optimized for web viewing
- Responsive design

## Prerequisites

1. **Install Rust**: If you haven't already, install Rust from https://rustup.rs/

2. **Add Web Target**:
   ```bash
   rustup target add wasm32-unknown-unknown
   ```

3. **Install Trunk**: Trunk is a wasm web application bundler for Rust.
   ```bash
   cargo install trunk
   ```

## Running the Example

### Development Mode (with auto-reload)

```bash
cd gcode_editor
trunk serve --open --example web_editor
```

This will:
- Build the web application
- Start a local development server (typically at http://127.0.0.1:8080)
- Open your default web browser
- Automatically rebuild when you make changes

### Production Build

```bash
cd gcode_editor
trunk build --release --example web_editor
```

The optimized build will be in the `dist/` directory, which you can deploy to any static web host.

## Native Testing

You can also run the example as a native desktop application for testing:

```bash
cd gcode_editor
cargo run --example web_editor
```

## Code Structure

The example (`web_editor.rs`) demonstrates:

1. **Web-specific initialization**: Uses `eframe::WebRunner` for web deployment
2. **Editor integration**: Embeds the gcode editor with full syntax highlighting
3. **UI enhancements**: Adds toolbar buttons, statistics panel, and tips
4. **Event handling**: Responds to editor events (content changes, line changes, etc.)
5. **Clipboard support**: Web-compatible copy functionality

## Customization

You can customize the editor by modifying the `WebEditorApp` struct:

- **Colors**: Modify `self.colors` to change syntax highlighting colors
- **Font size**: Change the `14.0` parameter in `show_editor()` call
- **Initial content**: Edit the default GCode in `Default::default()`
- **UI layout**: Modify the `update()` method to change the interface

## Deployment

The `dist/` directory contains all the files needed for deployment:
- `index.html` - The HTML page (copied from `webinterface/index.html` during build)
- `web_editor-*.wasm` - The compiled WebAssembly module
- `web_editor-*.js` - The JavaScript loader

You can deploy these files to:
- GitHub Pages
- Netlify
- Vercel
- Any static file hosting service

## Troubleshooting

**Build fails with "error: linker `lld` not found"**: Install the lld linker:
- Linux: `sudo apt install lld`
- macOS: `brew install llvm`

**Browser shows blank screen**: Check the browser console (F12) for error messages. Common issues:
- Missing wasm-bindgen dependencies
- Canvas element ID mismatch (must be "the_canvas_id")

**Performance issues**: Use `--release` flag for production builds to enable optimizations.
