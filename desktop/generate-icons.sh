#!/bin/bash
# Generate icons for all platforms from a source PNG
# Usage: ./generate-icons.sh <source-png>

set -e

SOURCE="${1:-icon.png}"
ICONS_DIR="src-tauri/icons"

if [ ! -f "$SOURCE" ]; then
    echo "Error: Source file '$SOURCE' not found"
    exit 1
fi

echo "Generating icons from $SOURCE..."

mkdir -p "$ICONS_DIR"

# Check for required tools
if command -v magick &> /dev/null; then
    CONVERT_CMD="magick"
elif command -v convert &> /dev/null; then
    CONVERT_CMD="convert"
else
    echo "Warning: ImageMagick not found. Install it for .ico generation."
    CONVERT_CMD=""
fi

# First, create a square version of the source image
SQUARE_SOURCE="$ICONS_DIR/icon_square_temp.png"
if [ -n "$CONVERT_CMD" ]; then
    echo "Creating square icon..."
    $CONVERT_CMD "$SOURCE" -background none -gravity center -extent 256x256 "PNG32:$SQUARE_SOURCE"
elif command -v sips &> /dev/null; then
    # macOS fallback - pad to square
    cp "$SOURCE" "$SQUARE_SOURCE"
else
    cp "$SOURCE" "$SQUARE_SOURCE"
fi

# Generate PNG icons of various sizes
echo "Generating PNG icons..."
if command -v sips &> /dev/null && [ -n "$CONVERT_CMD" ]; then
    # macOS with ImageMagick - use magick for RGBA format
    $CONVERT_CMD "$SQUARE_SOURCE" -resize 32x32 "PNG32:$ICONS_DIR/32x32.png"
    $CONVERT_CMD "$SQUARE_SOURCE" -resize 128x128 "PNG32:$ICONS_DIR/128x128.png"
    $CONVERT_CMD "$SQUARE_SOURCE" -resize 256x256 "PNG32:$ICONS_DIR/128x128@2x.png"
    $CONVERT_CMD "$SQUARE_SOURCE" -resize 512x512 "PNG32:$ICONS_DIR/icon.png"
elif [ -n "$CONVERT_CMD" ]; then
    # Linux/Windows with ImageMagick
    $CONVERT_CMD "$SQUARE_SOURCE" -resize 32x32 "PNG32:$ICONS_DIR/32x32.png"
    $CONVERT_CMD "$SQUARE_SOURCE" -resize 128x128 "PNG32:$ICONS_DIR/128x128.png"
    $CONVERT_CMD "$SQUARE_SOURCE" -resize 256x256 "PNG32:$ICONS_DIR/128x128@2x.png"
    $CONVERT_CMD "$SQUARE_SOURCE" -resize 512x512 "PNG32:$ICONS_DIR/icon.png"
else
    echo "Warning: No image processing tool found. Using source as-is."
    cp "$SOURCE" "$ICONS_DIR/icon.png"
fi

# Generate .ico for Windows
if [ -n "$CONVERT_CMD" ]; then
    echo "Generating Windows .ico..."
    $CONVERT_CMD "$SQUARE_SOURCE" -define icon:auto-resize=256,128,64,48,32,16 "$ICONS_DIR/icon.ico"
fi

# Generate .icns for macOS
if command -v iconutil &> /dev/null; then
    echo "Generating macOS .icns..."
    ICONSET_DIR="$ICONS_DIR/icon.iconset"
    mkdir -p "$ICONSET_DIR"

    if [ -n "$CONVERT_CMD" ]; then
        $CONVERT_CMD "$SQUARE_SOURCE" -resize 16x16 "PNG32:$ICONSET_DIR/icon_16x16.png"
        $CONVERT_CMD "$SQUARE_SOURCE" -resize 32x32 "PNG32:$ICONSET_DIR/icon_16x16@2x.png"
        $CONVERT_CMD "$SQUARE_SOURCE" -resize 32x32 "PNG32:$ICONSET_DIR/icon_32x32.png"
        $CONVERT_CMD "$SQUARE_SOURCE" -resize 64x64 "PNG32:$ICONSET_DIR/icon_32x32@2x.png"
        $CONVERT_CMD "$SQUARE_SOURCE" -resize 128x128 "PNG32:$ICONSET_DIR/icon_128x128.png"
        $CONVERT_CMD "$SQUARE_SOURCE" -resize 256x256 "PNG32:$ICONSET_DIR/icon_128x128@2x.png"
        $CONVERT_CMD "$SQUARE_SOURCE" -resize 256x256 "PNG32:$ICONSET_DIR/icon_256x256.png"
        $CONVERT_CMD "$SQUARE_SOURCE" -resize 512x512 "PNG32:$ICONSET_DIR/icon_256x256@2x.png"
        $CONVERT_CMD "$SQUARE_SOURCE" -resize 512x512 "PNG32:$ICONSET_DIR/icon_512x512.png"
        $CONVERT_CMD "$SQUARE_SOURCE" -resize 1024x1024 "PNG32:$ICONSET_DIR/icon_512x512@2x.png"
    else
        sips -z 16 16 "$SQUARE_SOURCE" --out "$ICONSET_DIR/icon_16x16.png"
        sips -z 32 32 "$SQUARE_SOURCE" --out "$ICONSET_DIR/icon_16x16@2x.png"
        sips -z 32 32 "$SQUARE_SOURCE" --out "$ICONSET_DIR/icon_32x32.png"
        sips -z 64 64 "$SQUARE_SOURCE" --out "$ICONSET_DIR/icon_32x32@2x.png"
        sips -z 128 128 "$SQUARE_SOURCE" --out "$ICONSET_DIR/icon_128x128.png"
        sips -z 256 256 "$SQUARE_SOURCE" --out "$ICONSET_DIR/icon_128x128@2x.png"
        sips -z 256 256 "$SQUARE_SOURCE" --out "$ICONSET_DIR/icon_256x256.png"
        sips -z 512 512 "$SQUARE_SOURCE" --out "$ICONSET_DIR/icon_256x256@2x.png"
        sips -z 512 512 "$SQUARE_SOURCE" --out "$ICONSET_DIR/icon_512x512.png"
        sips -z 1024 1024 "$SQUARE_SOURCE" --out "$ICONSET_DIR/icon_512x512@2x.png"
    fi

    iconutil -c icns "$ICONSET_DIR" -o "$ICONS_DIR/icon.icns"
    rm -rf "$ICONSET_DIR"
fi

# Clean up temp file
rm -f "$SQUARE_SOURCE"

echo "Done! Generated icons in $ICONS_DIR:"
ls -la "$ICONS_DIR"
