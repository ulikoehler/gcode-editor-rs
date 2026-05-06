#!/bin/bash
set -e

# Script to generate screenshots for all gcode example files.
# Uses egui_kittest for headless wgpu rendering — no X11 required.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
SCREENSHOT_DIR="$PROJECT_DIR/screenshots"
GCODE_DIR="$PROJECT_DIR/examples/gcode"

mkdir -p "$SCREENSHOT_DIR"

echo "Building screenshot_generator example..."
cd "$PROJECT_DIR"
cargo build --example screenshot_generator --release

for gcode_file in "$GCODE_DIR"/*.gcode; do
    filename=$(basename "$gcode_file")
    screenshot_name="${filename%.gcode}.png"
    screenshot_path="$SCREENSHOT_DIR/$screenshot_name"

    echo "Generating: $filename -> $screenshot_name"
    cargo run --release --example screenshot_generator -- "$gcode_file" "$screenshot_path"
done

echo ""
echo "Screenshot generation complete!"
ls -lh "$SCREENSHOT_DIR"
