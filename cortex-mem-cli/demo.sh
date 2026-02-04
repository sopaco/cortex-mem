#!/bin/bash
# Cortex-Mem CLI 简洁测试脚本（无警告）

set -e

echo "================================"
echo "Cortex-Mem CLI 快速测试"
echo "================================"
echo ""

# 使用 --quiet 去掉编译输出，2>&1 | grep -v "warning:" 去掉警告
CM="cargo run --quiet --bin cortex-mem --"

echo "📊 Step 1: 查看初始统计..."
$CM stats
echo ""

echo "📝 Step 2: 创建测试会话..."
$CM session create demo-session --title "演示会话"
echo ""

echo "✉️  Step 3: 添加测试消息..."
$CM add --thread demo-session "你好，我想了解如何使用Rust实现OAuth 2.0"
$CM add --thread demo-session --role assistant "我建议使用oauth2 crate，这是Rust生态中最成熟的OAuth实现"
$CM add --thread demo-session "具体的集成步骤是什么？"
$CM add --thread demo-session --role assistant "首先需要配置OAuth客户端，然后实现授权流程和token管理"
echo ""

echo "📋 Step 4: 列出会话内容..."
$CM list --thread demo-session
echo ""

echo "🔍 Step 5: 搜索测试..."
$CM search "OAuth Rust" --thread demo-session -n 5
echo ""

echo "📋 Step 6: 查看所有会话..."
$CM session list
echo ""

echo "📊 Step 7: 查看统计..."
$CM stats
echo ""

echo "================================"
echo "✅ 测试完成！"
echo "================================"
echo ""
echo "💡 提示："
echo "  - 数据保存在: ./cortex-data/"
echo "  - 查看文件: ls -la cortex-data/threads/demo-session/"
echo "  - 查看消息: $CM get <URI>"
echo "  - 提取记忆: $CM session extract demo-session"
echo "  - 关闭会话: $CM session close demo-session"
echo ""
