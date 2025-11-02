#!/bin/bash

echo "===================================="
echo "Building Esponquen Distribution"
echo "===================================="
echo ""

# Build in release mode
echo "[1/5] Building release binary..."
cargo build --release
if [ $? -ne 0 ]; then
    echo "ERROR: Build failed!"
    exit 1
fi
echo "✓ Build successful"
echo ""

# Create dist directory
echo "[2/5] Creating dist directory..."
rm -rf dist
mkdir -p dist
echo "✓ Created dist directory"
echo ""

# Copy the binary
echo "[3/5] Copying binary..."
cp target/release/esponquen dist/
if [ $? -ne 0 ]; then
    echo "ERROR: Failed to copy binary!"
    exit 1
fi
chmod +x dist/esponquen
echo "✓ Binary copied"
echo ""

# Copy model directory
echo "[4/5] Copying model directory..."
if [ -d "model" ]; then
    cp -r model dist/
    if [ $? -ne 0 ]; then
        echo "ERROR: Failed to copy model directory!"
        exit 1
    fi
    echo "✓ Model directory copied"
else
    echo "WARNING: model directory not found, skipping..."
fi
echo ""

# Copy icons directory
echo "[5/5] Copying icons directory..."
if [ -d "icons" ]; then
    cp -r icons dist/
    if [ $? -ne 0 ]; then
        echo "ERROR: Failed to copy icons directory!"
        exit 1
    fi
    echo "✓ Icons directory copied"
else
    echo "WARNING: icons directory not found, skipping..."
fi
echo ""

# Copy any shared libraries
echo "[OPTIONAL] Checking for shared libraries..."
if ldd target/release/esponquen | grep "=> /" | awk '{print $3}' | xargs -I '{}' test -f '{}'; then
    echo "✓ All shared libraries are system libraries"
else
    echo "Note: Binary depends on system libraries"
fi
echo ""

echo "===================================="
echo "Distribution package created successfully!"
echo "===================================="
echo ""
echo "Location: dist/esponquen"
echo ""
echo "Contents:"
ls -lh dist/
echo ""
echo "Run: ./dist/esponquen"
echo "Or with console: ./dist/esponquen --console"
echo ""
