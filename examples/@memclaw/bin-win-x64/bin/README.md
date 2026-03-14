# MemClaw Binaries for Windows x64

Place the following binaries in this directory:

- `qdrant.exe` - Qdrant vector database
- `cortex-mem-service.exe` - Cortex Memory HTTP service
- `cortex-mem-cli.exe` - Cortex Memory CLI tool

## Build from source

```powershell
# In cortex-mem project root
cargo build --release --target x86_64-pc-windows-msvc

# Copy binaries
copy target\x86_64-pc-windows-msvc\release\cortex-mem-service.exe bin\
copy target\x86_64-pc-windows-msvc\release\cortex-mem-cli.exe bin\
```

## Download Qdrant

Download from: https://github.com/qdrant/qdrant/releases
