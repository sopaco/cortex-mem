#!/bin/bash
# MCP Server Manual Test Script

echo "================================"
echo "Cortex-Mem MCP Server Manual Test"
echo "================================"
echo ""

MCP_BIN="./target/release/cortex-mem-mcp"

if [ ! -f "$MCP_BIN" ]; then
    echo "❌ MCP binary not found. Building..."
    cargo build --release --bin cortex-mem-mcp
fi

echo "📝 Testing MCP Server..."
echo ""

# Test 1: Initialize
echo "Test 1: Initialize"
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | $MCP_BIN 2>/dev/null &
PID=$!
sleep 1
kill $PID 2>/dev/null
echo "✓ Initialize test completed"
echo ""

# Test 2: Tools List
echo "Test 2: List Tools"
(echo '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' && sleep 0.5) | $MCP_BIN 2>/dev/null | jq -r '.result.tools[].name' 2>/dev/null
echo ""

# Test 3: Store Memory
echo "Test 3: Store Memory"
REQUEST='{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"store_memory","arguments":{"content":"测试记忆内容","thread_id":"test-mcp"}}}'
(echo "$REQUEST" && sleep 0.5) | $MCP_BIN 2>/dev/null | jq -r '.result.content[0].text' 2>/dev/null
echo ""

# Test 4: List Memories
echo "Test 4: List Memories"
REQUEST='{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"list_memories","arguments":{"thread_id":"test-mcp"}}}'
(echo "$REQUEST" && sleep 0.5) | $MCP_BIN 2>/dev/null | jq -r '.result.content[0].text' 2>/dev/null
echo ""

echo "================================"
echo "✅ Manual tests completed!"
echo "================================"
echo ""
echo "💡 For full integration testing:"
echo "   1. Configure Claude Desktop (see cortex-mem-mcp/README.md)"
echo "   2. Restart Claude Desktop"
echo "   3. Ask Claude to use cortex-mem tools"
echo ""
