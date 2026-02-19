#!/bin/bash
# Cortex-Mem CLI 快速测试脚本

set -e

echo "================================"
echo "Cortex-Mem CLI 快速测试"
echo "================================"
echo ""

# 设置别名以简化命令
alias cm='cargo run --quiet --bin cortex-mem --'

echo "📊 Step 1: 查看初始统计..."
cm stats
echo ""

echo "📝 Step 2: 创建测试会话..."
cm session create test-session --title "CLI测试会话"
echo ""

echo "✉️  Step 3: 添加测试消息..."
cm add --thread test-session "这是第一条测试消息"
cm add --thread test-session --role assistant "收到，这是助手的回复"
cm add --thread test-session "我们来讨论一下OAuth 2.0的实现"
cm add --thread test-session --role assistant "好的，OAuth 2.0建议使用授权码流程"
echo ""

echo "📋 Step 4: 列出会话内容..."
cm list --thread test-session
echo ""

echo "🔍 Step 5: 搜索测试..."
cm search "OAuth" --thread test-session -n 5
echo ""

echo "📋 Step 6: 查看所有会话..."
cm session list
echo ""

echo "🧠 Step 7: 提取记忆（注意：当前使用placeholder）..."
cm session extract test-session
echo ""

echo "🔒 Step 8: 关闭会话..."
cm session close test-session
echo ""

echo "📊 Step 9: 查看最终统计..."
cm stats
echo ""

echo "================================"
echo "✅ 测试完成！"
echo "================================"
echo ""
echo "数据已保存到: ./cortex-data/"
echo "查看文件结构: tree cortex-data/"
echo ""
