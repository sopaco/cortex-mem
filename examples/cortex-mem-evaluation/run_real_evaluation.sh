#!/bin/bash

# 切换到评估目录
cd "$(dirname "$0")"

# 生成数据集
echo "生成召回率数据集..."
cargo run -- generate-dataset --dataset-type recall --use-lab-data

echo "生成有效性数据集..."
cargo run -- generate-dataset --dataset-type effectiveness --use-lab-data

# 运行真实评估
echo "运行真实评估..."
cargo run -- run --config config/minimal_config.toml

echo "评估完成！结果保存在 results/ 目录中"
