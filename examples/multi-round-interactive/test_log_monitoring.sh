#!/bin/bash

# 测试日志监听功能的脚本

echo "🧪 开始测试日志监听功能..."

cd examples/multi-round-interactive

# 清理旧的日志文件
echo "🧹 清理旧的日志文件..."
rm -rf logs/*

# 启动程序并发送quit命令
echo "🚀 启动程序并发送quit命令..."
echo "/quit" | timeout 15s cargo run

echo "✅ 测试完成！"
echo "📋 检查生成的日志文件："
ls -la logs/

if [ -f logs/*.log ]; then
    echo "📄 最新日志文件内容："
    cat logs/*.log | head -20
fi