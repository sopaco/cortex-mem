# MemClaw Binaries for macOS Apple Silicon

Place the following binaries in this directory:

- `qdrant` - Qdrant vector database
- `cortex-mem-service` - Cortex Memory HTTP service  
- `cortex-mem-cli` - Cortex Memory CLI tool

## Build from source

```bash
# In cortex-mem project root
cargo build --release --target aarch64-apple-darwin

# Copy binaries
cp target/aarch64-apple-darwin/release/cortex-mem-service bin/
cp target/aarch64-apple-darwin/release/cortex-mem-cli bin/
```

## Download Qdrant

Download from: https://github.com/qdrant/qdrant/releases
