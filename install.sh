#!/bin/bash

set -e

echo "Building frc..."
cargo build --release

echo "Installing frc to /usr/local/bin..."
if [ -w /usr/local/bin ]; then
    cp target/release/frc /usr/local/bin/
else
    sudo cp target/release/frc /usr/local/bin/
fi

echo "Installation complete!"
echo ""
echo "Try running: frc --help"
