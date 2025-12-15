#!/bin/bash

# Cortex-Mem 评估框架运行示例
# 这个脚本展示了如何使用不同的命令运行评估

echo "=== Cortex-Mem 评估框架运行示例 ==="
echo ""

# 1. 生成数据集（使用实验室数据）
echo "1. 生成测试数据集（使用实验室数据）:"
echo "   cargo run -- generate-dataset --dataset-type all --size 50 --use-lab-data"
echo ""

# 2. 运行完整评估（模拟模式）
echo "2. 运行完整评估（模拟模式）:"
echo "   cargo run -- run --config config/evaluation_config.toml"
echo ""

# 3. 运行完整评估（真实模式 - 需要配置）
echo "3. 运行完整评估（真实模式）:"
echo "   cargo run -- run --config config/real_evaluation_config.toml"
echo "   注意：这需要配置 MemoryManager 和向量存储"
echo ""

# 4. 仅运行召回率评估
echo "4. 仅运行召回率评估:"
echo "   cargo run -- recall --config config/evaluation_config.toml"
echo ""

# 5. 仅运行有效性评估
echo "5. 仅运行有效性评估:"
echo "   cargo run -- effectiveness --config config/evaluation_config.toml"
echo ""

# 6. 仅运行性能评估
echo "6. 仅运行性能评估:"
echo "   cargo run -- performance --config config/evaluation_config.toml"
echo ""

# 7. 验证数据集
echo "7. 验证数据集:"
echo "   cargo run -- validate-dataset --dataset-path data/test_cases/lab_recall_dataset.json --dataset-type recall"
echo ""

echo "=== 快速开始 ==="
echo ""
echo "要快速测试框架，请按顺序运行以下命令："
echo ""
echo "1. 生成数据集:"
echo "   cargo run -- generate-dataset --dataset-type recall --size 20 --use-lab-data"
echo ""
echo "2. 运行模拟评估:"
echo "   cargo run -- run --config config/evaluation_config.toml"
echo ""
echo "3. 查看结果:"
echo "   ls -la results/"
echo "   cat results/evaluation.log | tail -20"
echo ""