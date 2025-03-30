#!/bin/bash
set -e

echo "==============================================="
echo "  Generating Documentation with cargo doc"
echo "==============================================="

# Generate documentation
cargo doc --no-deps

# Verify that documentation was generated successfully
if [ $? -eq 0 ]; then
    echo -e "\n✅ Documentation generated successfully!"
    echo "Open the documentation with: open target/doc/index.html"
else
    echo -e "\n❌ Documentation generation failed!"
    exit 1
fi
