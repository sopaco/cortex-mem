# Cortex-Mem 核心能力评估框架

一个用于评估 Cortex-Mem 核心能力的框架，重点验证召回率和记忆有效性。

## 功能特性

- **召回率评估**：验证向量检索的准确性和完整性
- **记忆有效性评估**：测试记忆提取、分类、去重、重要性评估等核心功能
- **性能基准测试**：测量响应时间、吞吐量等性能指标
- **自动化评估流程**：完整的端到端评估管道
- **可配置的测试**：支持自定义评估参数和测试数据集
- **详细报告**：生成多种格式的评估报告和可视化图表

## 项目结构

```
cortex-mem-evaluation/
├── Cargo.toml                    # Rust 项目配置
├── config/
│   └── evaluation_config.toml    # 评估配置文件
├── src/
│   ├── main.rs                   # 主程序入口
│   ├── evaluator/                # 评估器模块
│   │   ├── mod.rs
│   │   ├── metrics.rs           # 评估指标定义
│   │   ├── recall_evaluator.rs  # 召回率评估器
│   │   ├── effectiveness_evaluator.rs # 有效性评估器
│   │   └── performance_evaluator.rs   # 性能评估器
│   ├── dataset/                  # 数据集模块
│   │   ├── mod.rs
│   │   ├── generator.rs         # 测试数据生成器
│   │   ├── loader.rs           # 数据集加载器
│   │   └── types.rs            # 数据集类型定义
│   ├── runner/                  # 运行器模块
│   │   ├── mod.rs
│   │   ├── benchmark_runner.rs # 基准测试运行器
│   │   └── experiment_runner.rs # 实验运行器
│   └── report/                  # 报告模块
│       ├── mod.rs
│       ├── generator.rs        # 报告生成器
│       └── visualizer.rs       # 可视化工具
├── data/                        # 测试数据
│   ├── test_cases/             # 测试用例
│   └── ground_truth/           # 基准真值数据
├── scripts/                     # 脚本文件
│   ├── run_evaluation.sh       # 评估脚本
│   └── generate_report.sh      # 报告生成脚本
└── results/                     # 评估结果输出
    ├── reports/                # 评估报告
    └── visualizations/         # 可视化图表
```

## 快速开始

### 1. 安装依赖

确保已安装 Rust 工具链：

```bash
# 检查 Rust 安装
rustc --version
cargo --version
```

### 2. 构建项目

```bash
cd examples/cortex-mem-evaluation
cargo build --release
```

### 3. 生成测试数据集

```bash
# 使用脚本
./scripts/run_evaluation.sh --generate --size 100

# 或直接使用 cargo
cargo run -- generate-dataset --dataset-type all --size 100
```

### 4. 运行评估

```bash
# 运行完整评估
./scripts/run_evaluation.sh --mode all

# 仅运行召回率评估
./scripts/run_evaluation.sh --mode recall

# 仅运行有效性评估
./scripts/run_evaluation.sh --mode effectiveness

# 仅运行性能评估
./scripts/run_evaluation.sh --mode performance
```

### 5. 查看结果

评估结果将保存在 `results/` 目录中：

```bash
# 查看评估报告
ls -la results/reports/

# 查看 JSON 格式的详细结果
cat results/comprehensive_report.json | jq .
```

## 评估指标

### 召回率评估指标

- **Precision@K (P@K)**：前K个结果中的相关记忆比例
- **Recall@K (R@K)**：前K个结果中检索到的相关记忆比例
- **Mean Average Precision (MAP)**：平均精度均值
- **Normalized Discounted Cumulative Gain (NDCG)**：考虑排序质量的指标

### 记忆有效性评估指标

- **事实提取准确性**：精确率、召回率、F1分数
- **记忆分类准确性**：准确率、混淆矩阵
- **重要性评估合理性**：相关性分数、平均绝对误差
- **去重效果**：重复检测精确率、召回率、合并正确率
- **记忆更新正确性**：更新操作正确率、合并操作正确率

### 性能评估指标

- **延迟**：添加、搜索、更新记忆的平均响应时间
- **吞吐量**：每秒处理的记忆操作数
- **资源使用**：内存、CPU、磁盘使用情况
- **并发性能**：多用户场景下的表现
- **可扩展性**：不同规模记忆库下的性能变化

## 配置说明

评估配置位于 `config/evaluation_config.toml`：

```toml
[general]
mode = "all"  # 评估模式：recall, effectiveness, performance, all
output_dir = "results"
verbose = true
random_seed = 42

[recall_evaluation]
enabled = true
k_values = [1, 3, 5, 10]
similarity_thresholds = [0.7, 0.8, 0.9]
max_results_per_query = 20

[effectiveness_evaluation]
enabled = true
verify_fact_extraction = true
verify_classification = true
verify_importance_evaluation = true
verify_deduplication = true
verify_memory_update = true
importance_score_tolerance = 1

[performance_evaluation]
enabled = true
memory_sizes = [100, 1000, 5000]
concurrent_users = [1, 5, 10]
test_duration_seconds = 30
```

## 命令行使用

### 主命令

```bash
cortex-mem-evaluation [命令] [选项]
```

### 可用命令

```bash
# 运行完整评估
cortex-mem-evaluation run --config config/evaluation_config.toml

# 仅运行召回率评估
cortex-mem-evaluation recall --config config/evaluation_config.toml

# 仅运行有效性评估
cortex-mem-evaluation effectiveness --config config/evaluation_config.toml

# 仅运行性能评估
cortex-mem-evaluation performance --config config/evaluation_config.toml

# 生成测试数据集
cortex-mem-evaluation generate-dataset --dataset-type all --size 100

# 验证测试数据集
cortex-mem-evaluation validate-dataset --dataset-path data/test_cases/recall_test_cases.json --dataset-type recall
```

### 选项

- `-c, --config <FILE>`：配置文件路径（默认：config/evaluation_config.toml）
- `-o, --output-dir <DIR>`：输出目录（默认：results）
- `--size <SIZE>`：数据集大小（默认：100）
- `-h, --help`：显示帮助信息

## 自定义评估

### 1. 创建自定义测试数据集

```rust
use cortex_mem_evaluation::dataset::DatasetGenerator;

let mut generator = DatasetGenerator::new(GeneratorConfig {
    dataset_size: 500,
    avg_relevant_memories: 5.0,
    ..Default::default()
});

let dataset = generator.generate_recall_dataset()?;
generator.save_dataset(&dataset, "my_custom_dataset.json")?;
```

### 2. 自定义评估配置

```toml
# my_evaluation_config.toml
[recall_evaluation]
k_values = [1, 5, 10, 20]
similarity_thresholds = [0.6, 0.75, 0.85, 0.95]

[performance_evaluation]
memory_sizes = [500, 2000, 10000]
concurrent_users = [1, 10, 50, 100]
```

### 3. 扩展评估器

```rust
use cortex_mem_evaluation::evaluator::{RecallEvaluator, RecallEvaluationConfig};

let config = RecallEvaluationConfig {
    k_values: vec![1, 3, 5, 10, 20],
    similarity_thresholds: vec![0.7, 0.8, 0.9],
    max_results_per_query: 50,
    save_detailed_results: true,
};

let evaluator = RecallEvaluator::new(config);
let metrics = evaluator.evaluate(&memory_manager, &dataset).await?;
```

## 开发指南

### 添加新的评估指标

1. 在 `src/evaluator/metrics.rs` 中定义新的指标结构
2. 在相应的评估器中实现计算逻辑
3. 更新报告生成器以包含新指标

### 添加新的测试数据集类型

1. 在 `src/dataset/types.rs` 中定义新的数据集结构
2. 在 `src/dataset/generator.rs` 中实现数据生成逻辑
3. 在 `src/dataset/loader.rs` 中实现数据加载逻辑

### 运行测试

```bash
# 运行单元测试
cargo test

# 运行集成测试
cargo test --test integration

# 运行特定测试
cargo test test_recall_evaluation
```

## 故障排除

### 常见问题

1. **构建失败**：确保 Rust 版本 >= 1.70，并检查依赖项
2. **数据集生成失败**：检查输出目录权限和磁盘空间
3. **评估运行失败**：检查配置文件语法和路径正确性
4. **内存不足**：减少测试数据集大小或调整性能测试参数

### 调试模式

启用详细日志：

```bash
RUST_LOG=debug ./scripts/run_evaluation.sh --mode all --verbose
```

或直接设置环境变量：

```bash
export RUST_LOG=debug
cargo run -- run --config config/evaluation_config.toml
```

## 贡献指南

1. Fork 项目仓库
2. 创建功能分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'Add amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 创建 Pull Request

## 许可证

本项目采用 MIT 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情。

## 联系方式

如有问题或建议，请通过以下方式联系：

- 项目 Issues：https://github.com/sopaco/cortex-mem/issues
- 电子邮件：cortex-mem@example.com

## 更新日志

### v0.1.0 (2025-12-09)
- 初始版本发布
- 实现召回率评估器
- 实现有效性评估器
- 实现性能评估器
- 添加测试数据集生成器
- 添加自动化评估脚本
- 添加配置系统和报告生成