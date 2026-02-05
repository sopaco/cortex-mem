#!/bin/bash

echo "========================================="
echo "Testing cortex-mem-tars configuration"
echo "========================================="
echo ""

# Clean up old processes
pkill -f cortex-mem-tars 2>/dev/null
sleep 1

echo "Current directory: $(pwd)"
echo ""

# Check for config.toml
if [ -f "config.toml" ]; then
    echo "✓ Found config.toml in current directory"
    echo ""
    echo "LLM configuration:"
    grep -A 5 "^\[llm\]" config.toml
else
    echo "✗ No config.toml found in current directory"
fi

echo ""
echo "========================================="
echo "Starting cortex-mem-tars..."
echo "========================================="
echo ""
echo "Watch for output like:"
echo "  ✓ Using config.toml from current directory: ..."
echo "  ✓ Successfully loaded config from: ..."
echo ""

cargo run -p cortex-mem-tars
