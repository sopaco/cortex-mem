# Cortex-Mem Service V2 快速测试脚本

# 设置基础URL
BASE_URL="http://localhost:8080"

echo "==================================="
echo "Cortex-Mem Service V2 API 测试"
echo "==================================="
echo ""

# 1. 健康检查
echo "1️⃣  健康检查"
curl -s "$BASE_URL/health" | jq '.'
echo ""
echo ""

# 2. 创建会话
echo "2️⃣  创建会话"
THREAD_ID="test-session-$(date +%s)"
curl -s -X POST "$BASE_URL/api/v2/sessions" \
  -H "Content-Type: application/json" \
  -d "{\"thread_id\": \"$THREAD_ID\", \"title\": \"Test Session\"}" | jq '.'
echo ""
echo ""

# 3. 添加用户消息
echo "3️⃣  添加用户消息"
curl -s -X POST "$BASE_URL/api/v2/sessions/$THREAD_ID/messages" \
  -H "Content-Type: application/json" \
  -d '{"role": "user", "content": "Hello! This is a test message."}' | jq '.'
echo ""
echo ""

# 4. 添加助手回复
echo "4️⃣  添加助手回复"
curl -s -X POST "$BASE_URL/api/v2/sessions/$THREAD_ID/messages" \
  -H "Content-Type: application/json" \
  -d '{"role": "assistant", "content": "Hi! I received your test message. How can I help you?"}' | jq '.'
echo ""
echo ""

# 5. 列出所有会话
echo "5️⃣  列出所有会话"
curl -s "$BASE_URL/api/v2/sessions" | jq '.'
echo ""
echo ""

# 6. 浏览文件系统
echo "6️⃣  浏览文件系统 (threads目录)"
curl -s "$BASE_URL/api/v2/filesystem?uri=cortex://threads" | jq '.data | .[0:3]'
echo ""
echo ""

# 7. 搜索消息
echo "7️⃣  搜索消息 (关键词: test)"
curl -s -X POST "$BASE_URL/api/v2/search" \
  -H "Content-Type: application/json" \
  -d "{\"query\": \"test\", \"thread\": \"$THREAD_ID\", \"limit\": 5}" | jq '.'
echo ""
echo ""

# 8. 关闭会话
echo "8️⃣  关闭会话"
curl -s -X POST "$BASE_URL/api/v2/sessions/$THREAD_ID/close" | jq '.'
echo ""
echo ""

echo "==================================="
echo "✅ 测试完成!"
echo "Thread ID: $THREAD_ID"
echo "==================================="
echo ""
echo "如果需要测试记忆提取功能，请先设置LLM环境变量，然后运行："
echo "curl -X POST $BASE_URL/api/v2/automation/extract/$THREAD_ID -H 'Content-Type: application/json' -d '{\"auto_save\": false}' | jq '.'"
