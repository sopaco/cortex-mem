# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2025-06-18

### Changed

- **Breaking Change**: Refactored the tool interface from a single multi-action tool to four distinct tools
- Updated `MemoryTool` to `MemoryTools` class that provides individual tool instances
- Each tool now matches its corresponding MCP protocol tool with identical parameters and behavior

### Added

- `StoreMemoryTool` - Dedicated tool for storing new memories
- `QueryMemoryTool` - Dedicated tool for searching memories using semantic similarity
- `ListMemoriesTool` - Dedicated tool for listing memories with filtering
- `GetMemoryTool` - Dedicated tool for retrieving a specific memory by ID
- `MemoryToolOutput` - Common output structure for all tools
- Individual argument structures for each tool (`StoreMemoryArgs`, `QueryMemoryArgs`, etc.)
- `create_memory_tools()` function replacing `create_memory_tool()`

### Deprecated

- `MemoryTool` and `create_memory_tool()` are deprecated but still functional for backward compatibility
- The old single-tool interface with action parameter is deprecated in favor of individual tools

### Fixed

- Improved type safety with dedicated argument structures
- Simplified tool definitions by reusing MCP tool definitions from cortex-mem-tools

## [0.1.0] - Initial Release

- Initial implementation with a single multi-action tool (store, search, recall, get)
- Integration with RIG AI agent framework