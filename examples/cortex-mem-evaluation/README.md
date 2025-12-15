# Cortex-Mem Evaluation Framework

Cortex-Mem核心能力评估框架，用于测试和验证Cortex-Mem记忆管理系统的性能、准确性和功能完整性。

## 项目概述

本评估框架旨在解决原始"空壳"实现问题，提供真实的、基于实际MemoryManager API调用的评估能力。框架支持两种评估模式：

1. **模拟评估模式** - 验证框架功能，无需实际MemoryManager实例
2. **真实评估模式** - 基于实际cortex-mem-core API的真实评估

## 核心功能

### 1. 召回率评估 (Recall Evaluation)
- 测试向量检索的准确性和完整性
- 支持指标：Precision@K, Recall@K, MAP, NDCG
- 可配置的相似度阈值和K值

### 2. 记忆有效性评估 (Effectiveness Evaluation)
- 事实提取准确性测试
- 记忆分类正确性验证
- 重要性评分合理性评估
- 去重和更新逻辑检查

### 3. 性能评估 (Performance Evaluation)
- 基准测试：基本操作性能
- 负载测试：并发用户性能
- 压力测试：系统极限性能
- 可扩展性测试：不同规模性能

## 快速开始

### 环境要求
- Rust 1.70+
- Cargo包管理器
- Cortex-Mem项目依赖

### 安装和运行

```bash
# 进入评估项目目录
cd /Users/jiangmeng/workspace/SAW/cortex-mem/examples/cortex-mem-evaluation

# 查看所有可用命令
cargo run -- --help

# 生成测试数据集（使用实验室数据）
cargo run -- generate-dataset

# 运行完整评估（模拟模式）
cargo run -- run --config config/evaluation_config.toml

# 仅运行召回率评估
cargo run -- recall --config config/evaluation_config.toml

# 仅运行有效性评估
cargo run -- effectiveness --config config/evaluation_config.toml
```

## 详细使用指南

### 1. 数据准备

#### 实验室数据源
框架集成了实验室真实数据，位于`data/lab/`目录：
- `conversations.json` - 对话数据（JSON格式）
- `technical_docs.csv` - 技术文档（CSV格式）
- `business_reports.txt` - 商业报告（文本格式）

#### 生成测试数据集
```bash
# 生成默认数据集（使用实验室数据）
cargo run -- generate-dataset

# 生成特定大小的数据集
cargo run -- generate-dataset --size 100

# 生成特定类型的数据集
cargo run -- generate-dataset --type recall
cargo run -- generate-dataset --type effectiveness
```

生成的数据集保存在`data/test_cases/`目录：
- `lab_recall_dataset.json` - 召回率测试集
- `lab_effectiveness_dataset.json` - 有效性测试集

### 2. 配置文件说明

#### 主配置文件 (`config/evaluation_config.toml`)
```toml
[general]
mode = "all"  # all, recall, effectiveness, performance
dataset_size = 100
save_detailed_results = true
use_real_evaluators = false  # 是否使用真实评估器

[recall_evaluation]
k_values = [1, 3, 5, 10]
similarity_thresholds = [0.7, 0.8, 0.9]
max_results_per_query = 20
use_real_evaluator = false

[effectiveness_evaluation]
verify_fact_extraction = true
verify_classification = true
verify_importance_evaluation = true
verify_deduplication = true
verify_memory_update = true
importance_score_tolerance = 1
use_real_evaluator = false
```

#### 真实评估配置 (`config/real_evaluation_config.toml`)
```toml
[general]
mode = "all"
dataset_size = 50
save_detailed_results = true
use_real_evaluators = true  # 启用真实评估器

[recall_evaluation]
use_real_evaluator = true
test_cases_path = "data/test_cases/lab_recall_dataset.json"

[effectiveness_evaluation]
use_real_evaluator = true
test_cases_path = "data/test_cases/lab_effectiveness_dataset.json"
```

### 3. 评估模式

#### 模拟评估模式
- 使用模拟数据测试框架功能
- 不需要实际的MemoryManager实例
- 适合验证评估框架本身

```bash
# 运行模拟评估
cargo run -- run --config config/evaluation_config.toml
```

#### 真实评估模式
- 需要实际的MemoryManager实例
- 测试真实的cortex-mem-core API调用
- 提供真实的性能指标

```bash
# 运行真实评估
cargo run -- run --config config/real_evaluation_config.toml
```

### 4. 输出结果

评估结果保存在`results/`目录：

```
results/
├── reports/                      # 评估报告
│   ├── recall_report.md          # 召回率报告
│   ├── effectiveness_report.md   # 有效性报告
│   ├── simulation_report.md      # 模拟评估报告
│   └── real_evaluation_report.md # 真实评估报告
├── visualizations/               # 可视化图表
│   ├── example_chart_data.json   # 示例图表数据
│   └── README.md                 # 图表说明
└── *.json                        # 原始评估数据
```

## 项目架构

### 目录结构
```
cortex-mem-evaluation/
├── Cargo.toml                    # 项目配置
├── README.md                     # 本文档
├── config/                       # 配置文件
│   ├── evaluation_config.toml        # 主评估配置
│   ├── real_evaluation_config.toml   # 真实评估配置
│   └── memory_manager_config.toml    # MemoryManager配置
├── data/                         # 数据目录
│   ├── lab/                          # 实验室数据源
│   ├── test_cases/                   # 生成的测试数据集
│   └── ground_truth/                 # 基准真值数据
├── src/                          # 源代码
│   ├── main.rs                      # 主程序入口
│   ├── evaluator/                   # 评估器模块
│   │   ├── mod.rs                       # 模块导出
│   │   ├── metrics.rs                   # 评估指标定义
│   │   ├── recall_evaluator.rs          # 召回率评估器
│   │   ├── effectiveness_evaluator.rs   # 有效性评估器
│   │   ├── real_recall_evaluator.rs     # 真实召回率评估器
│   │   └── real_effectiveness_evaluator.rs # 真实有效性评估器
│   ├── dataset/                    # 数据集模块
│   │   ├── mod.rs                       # 模块导出
│   │   ├── types.rs                     # 数据类型定义
│   │   ├── generator.rs                 # 数据集生成器
│   │   ├── loader.rs                    # 数据集加载器
│   │   └── lab_data_integration.rs      # 实验室数据集成
│   ├── runner/                     # 运行器模块
│   │   ├── mod.rs                       # 模块导出
│   │   ├── experiment_runner.rs         # 实验运行器
│   │   └── benchmark_runner.rs          # 基准测试运行器
│   ├── report/                     # 报告模块
│   │   ├── mod.rs                       # 模块导出
│   │   ├── generator.rs                 # 报告生成器
│   │   └── visualizer.rs                # 可视化工具
│   └── memory/                     # 记忆管理器模块
│       ├── mod.rs                       # 模块导出
│       ├── in_memory_vector_store.rs    # 内存向量存储
│       └── mock_llm_client.rs           # 模拟LLM客户端
├── results/                       # 评估结果输出
├── scripts/                       # 脚本目录
│   └── run_evaluation.sh              # 评估运行脚本
└── examples/                      # 示例代码
    └── basic_integration.rs           # 基本集成示例
```

### 核心模块说明

#### 1. 评估器模块 (`src/evaluator/`)
- **RecallEvaluator** - 模拟召回率评估器
- **EffectivenessEvaluator** - 模拟有效性评估器
- **RealRecallEvaluator** - 真实召回率评估器（基于MemoryManager）
- **RealEffectivenessEvaluator** - 真实有效性评估器（基于MemoryManager）

#### 2. 数据集模块 (`src/dataset/`)
- **DatasetGenerator** - 数据集生成器
- **DatasetLoader** - 数据集加载器
- **LabDataIntegrator** - 实验室数据集成器

#### 3. 实验运行器 (`src/runner/`)
- **ExperimentRunner** - 主实验运行器
- 协调完整的评估流程
- 支持多种评估模式

#### 4. 记忆管理器模块 (`src/memory/`)
- **InMemoryVectorStore** - 内存向量存储实现
- **MockLLMClient** - 模拟LLM客户端
- **create_simple_memory_manager()** - 创建简单MemoryManager实例

## 高级用法

### 集成实际MemoryManager

要运行真实评估，需要集成实际的MemoryManager实例：

```rust
use cortex_mem_core::MemoryManager;
use std::sync::Arc;

// 创建MemoryManager实例
let memory_manager = Arc::new(MemoryManager::new(config));

// 创建真实评估器
let recall_evaluator = RealRecallEvaluator::new(
    recall_config.clone(),
    memory_manager.clone(),
);

let effectiveness_evaluator = RealEffectivenessEvaluator::new(
    effectiveness_config.clone(),
    memory_manager,
);
```

### 自定义数据集

#### 添加新的数据源
1. 在`data/lab/`目录添加新的数据文件
2. 更新`src/dataset/lab_data_integration.rs`中的`create_example_lab_config()`函数
3. 重新生成数据集

#### 创建自定义数据集生成器
```rust
use crate::dataset::{DatasetGenerator, RecallTestDataset};

let mut generator = DatasetGenerator::new(config);
let dataset = generator.generate_custom_dataset()?;
```

### 扩展评估指标

在`src/evaluator/metrics.rs`中添加新的指标类型：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomMetrics {
    pub custom_score: f64,
    pub detailed_breakdown: HashMap<String, f64>,
}
```

## 故障排除

### 常见问题

#### 1. 数据文件缺失错误
```bash
Error: 读取JSON文件失败: data/lab/conversations.json
```
**解决方案**：
```bash
# 重新创建数据目录和文件
mkdir -p data/lab data/test_cases data/ground_truth
# 运行生成数据集命令
cargo run -- generate-dataset
```

#### 2. 配置文件找不到
```bash
Error: 加载配置失败
```
**解决方案**：
```bash
# 确保在正确目录运行
cd /Users/jiangmeng/workspace/SAW/cortex-mem/examples/cortex-mem-evaluation
# 使用相对路径
cargo run -- run --config config/evaluation_config.toml
```

#### 3. 编译警告
框架包含一些未使用的导入和变量警告，这些不影响功能。如需清理：
```bash
cargo fix --bin "cortex-mem-evaluation"
```

### 调试模式

启用详细日志：
```bash
RUST_LOG=debug cargo run -- run --config config/evaluation_config.toml
```

## 开发指南

### 添加新的评估类型

1. 在`src/evaluator/`目录创建新的评估器
2. 实现评估接口
3. 在`src/evaluator/mod.rs`中导出
4. 更新实验运行器以支持新的评估类型

### 修改评估逻辑

主要评估逻辑位于：
- `src/evaluator/real_recall_evaluator.rs` - 真实召回率评估
- `src/evaluator/real_effectiveness_evaluator.rs` - 真实有效性评估

### 测试修改

```bash
# 运行编译检查
cargo check

# 运行单元测试
cargo test

# 运行完整测试流程
./scripts/run_evaluation.sh --test
```

## 性能优化建议

1. **数据集大小**：根据测试需求调整数据集大小
2. **并行处理**：启用`enable_parallel_evaluation`配置
3. **内存管理**：合理设置`max_results_per_query`参数
4. **缓存策略**：考虑添加结果缓存机制

## 贡献指南

1. 遵循现有代码风格和架构模式
2. 添加适当的测试用例
3. 更新相关文档
4. 确保向后兼容性

## 许可证

本项目基于Cortex-Mem项目的许可证。

## 联系方式

如有问题或建议，请参考Cortex-Mem项目的主仓库。

---

*最后更新：2025年12月14日*