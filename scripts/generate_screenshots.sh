#!/bin/bash
set -e

# Script to generate screenshots for all gcode example files
# Uses Xvfb for headless operation and captures only the egui window

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
SCREENSHOT_DIR="$PROJECT_DIR/screenshots"
GCODE_DIR="$PROJECT_DIR/examples/gcode"

# Create screenshots directory
mkdir -p "$SCREENSHOT_DIR"

# Xvfb configuration
XVFB_DISPLAY=:99
XVFB_WIDTH=1024
XVFB_HEIGHT=768
XVFB_DEPTH=24

echo "Generating screenshots in: $SCREENSHOT_DIR"

# Check if required tools are available
if ! command -v Xvfb &> /dev/null; then
    echo "Error: 'Xvfb' is required but not installed."
    echo "Install with: sudo apt-get install xvfb"
    exit 1
fi

if ! command -v import &> /dev/null; then
    echo "Error: 'import' (ImageMagick) is required but not installed."
    echo "Install with: sudo apt-get install imagemagick"
    exit 1
fi

if ! command -v xwininfo &> /dev/null; then
    echo "Error: 'xwininfo' is required but not installed."
    echo "Install with: sudo apt-get install x11-utils"
    exit 1
fi

# Build the screenshot_generator example first
echo "Building screenshot_generator example..."
cd "$PROJECT_DIR"
cargo build --example screenshot_generator --release

# Start Xvfb with software rendering to avoid GPU issues
echo "Starting Xvfb on display $XVFB_DISPLAY..."
LIBGL_ALWAYS_SOFTWARE=1 Xvfb "$XVFB_DISPLAY" -screen 0 "${XVFB_WIDTH}x${XVFB_HEIGHT}x${XVFB_DEPTH}" -ac +render -noreset &
XVFB_PID=$!

# Wait for Xvfb to start
sleep 2

# Set DISPLAY environment variable
export DISPLAY="$XVFB_DISPLAY"

# Function to cleanup
cleanup() {
    echo "Cleaning up..."
    kill $XVFB_PID 2>/dev/null || true
    wait $XVFB_PID 2>/dev/null || true
}

trap cleanup EXIT

# Generate O100 screenshot with light theme first
echo "Generating O100 screenshot with light theme..."
cargo run --release --example screenshot_generator -- --screenshot-o100 &
APP_PID=$!

# Wait for the window to appear and render
echo "  Waiting for window to appear..."
sleep 3

# Find the window using xwininfo
WINDOW_ID=$(xwininfo -root -tree | grep -i "gcode editor" | head -1 | awk '{print $1}')

if [ -z "$WINDOW_ID" ]; then
    WINDOW_ID=$(xwininfo -root -children | grep -o '0x[0-9a-f]*' | tail -1)
fi

if [ -n "$WINDOW_ID" ]; then
    echo "  Found window ID: $WINDOW_ID"
    import -window "$WINDOW_ID" -silent "$SCREENSHOT_DIR/ocodes_examples_light_theme.png"
    echo "  Screenshot saved to: $SCREENSHOT_DIR/ocodes_examples_light_theme.png"
else
    echo "  Warning: Could not find window, using screen capture fallback"
    import -window root -silent "$SCREENSHOT_DIR/ocodes_examples_light_theme.png"
    echo "  Screenshot saved to: $SCREENSHOT_DIR/ocodes_examples_light_theme.png"
fi

# Kill the application
echo "  Closing application..."
kill $APP_PID 2>/dev/null || true
wait $APP_PID 2>/dev/null || true

sleep 1

# Find all .gcode files in the gcode directory
GCODE_FILES=("$GCODE_DIR"/*.gcode)

# Sort files for consistent ordering
IFS=$'\n' GCODE_FILES=($(sort <<<"${GCODE_FILES[*]}"))
unset IFS

for gcode_file in "${GCODE_FILES[@]}"; do
    filename=$(basename "$gcode_file")
    screenshot_name="${filename%.gcode}.png"
    screenshot_path="$SCREENSHOT_DIR/$screenshot_name"
    
    echo "Processing: $filename"
    
    # Run the screenshot generator in the background
    cargo run --release --example screenshot_generator -- "$gcode_file" &
    APP_PID=$!
    
    # Wait for the window to appear and render
    echo "  Waiting for window to appear..."
    sleep 3
    
    # Find the window using xwininfo
    # Search for windows containing "GCode Editor" in the title
    WINDOW_ID=$(xwininfo -root -tree | grep -i "gcode editor" | head -1 | awk '{print $1}')
    
    if [ -z "$WINDOW_ID" ]; then
        # Try alternative method: get the most recently created window
        WINDOW_ID=$(xwininfo -root -children | grep -o '0x[0-9a-f]*' | tail -1)
    fi
    
    if [ -n "$WINDOW_ID" ]; then
        echo "  Found window ID: $WINDOW_ID"
        # Capture the window using import with window ID
        import -window "$WINDOW_ID" -silent "$screenshot_path"
        echo "  Screenshot saved to: $screenshot_path"
    else
        echo "  Warning: Could not find window, using screen capture fallback"
        # Fallback: capture entire virtual screen
        import -window root -silent "$screenshot_path"
        echo "  Screenshot saved to: $screenshot_path"
    fi
    
    # Kill the application
    echo "  Closing application..."
    kill $APP_PID 2>/dev/null || true
    wait $APP_PID 2>/dev/null || true
    
    # Small delay between screenshots
    sleep 1
done

echo ""
echo "Screenshot generation complete!"
echo "Screenshots saved to: $SCREENSHOT_DIR"
ls -lh "$SCREENSHOT_DIR"
