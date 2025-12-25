# Cortex Mem 评估系统

## 项目概述

本评估系统是专为 Cortex Mem 记忆管理系统设计的专业评估框架，提供数据集、评估指标、基线对比和统计分析的完整解决方案。

## 核心特性

- 📊 **专业评估指标**: Recall@K, Precision@K, MRR, NDCG 等记忆系统专用指标
- 🗄️ **增强数据集**: 50 个对话，150 个问题，涵盖多种场景
- 📈 **统计分析**: 95% 置信区间、标准差、分类统计
- 🤖 **基线对比**: 内置简单 RAG 基线用于性能对比
- 🔧 **模块化设计**: 清晰的组件分离，易于扩展和维护
- 🛡️ **稳定性保障**: 指数退避重试、详细日志、错误恢复
- 📄 **HTML报告**: 美观的可视化报告，包含图表和表格

## 项目架构

```
lomoco-evaluation/
├── src/cortex_mem/              # Cortex Mem 专用模块
│   ├── add.py                   # 记忆添加（含重试+统计）
│   ├── search.py                # 记忆搜索
│   └── config_utils.py          # 配置管理工具
├── dataset/                     # 数据集目录
│   ├── locomo10.json            # 小型测试数据集 (10 对话)
│   └── locomo50.json            # 主要评估数据集 (50 对话, 150 问题)
├── metrics/                     # 评估指标模块
│   ├── memory_evaluation.py      # 记忆系统专用评估指标
│   ├── improved_llm_judge.py   # 改进的 LLM 评判器 (0-5 分评分)
│   └── utils.py                # 辅助工具函数
├── baselines/                   # 基线对比系统
│   └── simple_rag.py          # 简单 RAG 基线
├── results/                     # 结果输出目录
├── config.toml                 # 主配置文件
├── generate_enhanced_dataset.py # 数据集生成器
├── generate_report.py           # HTML报告生成器
├── run_cortex_mem_evaluation.py  # 主评估脚本
└── README.md                   # 本文档
```

## 快速开始

### 1. 环境准备

确保系统已安装：
- Rust 和 Cargo
- Python 3.8+
- Qdrant 向量数据库
- 必需的 Python 包: `pip install openai httpx toml tqdm jinja2 sentence-transformers scipy numpy`

### 2. 启动 Qdrant 服务

```bash
# macOS: 使用 Homebrew 安装
brew install qdrant

# Linux: 使用 Docker
docker run -p 6333:6333 -p 6334:6334 qdrant/qdrant

# 或直接下载二进制文件
# 访问 https://github.com/qdrant/qdrant/releases
```

启动 Qdrant:

```bash
# gRPC 模式（推荐）
qdrant --host 0.0.0.0 --port 6334

# 验证健康状态
curl http://localhost:6334/health
```

### 3. 配置 API 密钥

编辑 `config.toml` 文件，配置你的 API 密钥：

```toml
[llm]
api_base_url = "https://wanqing-api.corp.kuaishou.com/api/gateway/v1/endpoints"
api_key = "your_api_key"
model_efficient = "your_model_name"

[embedding]
api_base_url = "https://wanqing-api.corp.kuaishou.com/api/gateway/v1/endpoints"
model_name = "your_embedding_model"
api_key = "your_api_key"

[qdrant]
url = "http://localhost:6334"
```

## 评估流程

### 方式一：使用 Cortex Mem 评估

```bash
# 1. 添加记忆到 Cortex Mem
python run_cortex_mem_evaluation.py --method add --data dataset/locomo50.json

# 2. 搜索记忆并生成答案
python run_cortex_mem_evaluation.py --method search --data dataset/locomo50.json --top_k 10

# 3. 评估结果
python -m metrics.memory_evaluation \
  --results results/cortex_mem_results.json \
  --dataset dataset/locomo50.json \
  --output results/cortex_mem_evaluation.json

# 4. 生成HTML报告（推荐）
python generate_report.py \
  --results results/cortex_mem_evaluation.json \
  --output results/report.html
```

### 方式二：使用基线对比

```bash
# 1. 运行简单 RAG 基线
python baselines/simple_rag.py \
  --data dataset/locomo50.json \
  --output results/simple_rag_results.json \
  --top_k 10

# 2. 评估基线结果
python -m metrics.memory_evaluation \
  --results results/simple_rag_results.json \
  --dataset dataset/locomo50.json \
  --output results/cortex_mem_evaluated.json

# 3. 生成HTML报告
python generate_report.py \
  --results results/cortex_mem_evaluated.json \
  --output results/cortex_mem_evaluated.html
```

### 生成自定义数据集

```bash
# 生成 100 个对话的数据集
python generate_enhanced_dataset.py
```

## 评估指标说明

### 检索质量指标

| 指标 | 描述 | 评估内容 |
|--------|------|----------|
| **Recall@K** | Top K 结果中至少包含一个相关记忆的概率 | 检索覆盖率 |
| **Precision@K** | Top K 结果中相关记忆的比例 | 检索精确度 |

### 排名质量指标

| 指标 | 描述 | 评估内容 |
|--------|------|----------|
| **MRR** (Mean Reciprocal Rank) | 第一个相关记忆排名的倒数平均值（1.0 表示相关记忆在第一位） | 排名准确性 |
| **NDCG@K** | 考虑排序位置的归一化折损累计增益 | 综合排名质量 |

### 答案质量指标

| 指标 | 描述 | 评估内容 |
|--------|------|----------|
| **语义相似度** | 使用 Sentence BERT 计算的相似度 | 语义接近程度 |
| **关键词 F1** | 基于关键词重叠的 F1 分数 | 内容相关性 |
| **精确匹配** | 答案是否完全一致 | 严格准确率 |

### 统计指标

- **均值 (Mean)**: 指标的平均水平
- **标准差 (Std)**: 结果的稳定性
- **95% 置信区间**: 结果的统计显著性
- **分类统计**: 按问题类型 (category 1-5) 分组分析

## 结果解读

### 优秀级别

| 指标范围 | Recall@1 | Precision@1 | MRR | 整体评价 |
|----------|-----------|--------------|-----|----------|
| 优秀 | > 0.9 | > 0.9 | > 0.9 | 🟢 系统表现优异 |
| 良好 | 0.7-0.9 | 0.7-0.9 | 0.7-0.9 | 🟡 系统表现良好 |
| 一般 | 0.5-0.7 | 0.5-0.7 | 0.5-0.7 | 🟠 系统表现一般 |
| 需改进 | < 0.5 | < 0.5 | < 0.5 | 🔴 系统需要优化 |

## HTML 报告

使用 `generate_report.py` 生成美观的 HTML 报告，包含：

- 📊 总体指标概览（卡片布局）
- 📈 指标对比表格（按类别分组）
- 📂 分类指标详情（Grid 布局）
- 📖 指标定义和说明
- 🎨 可视化图表（进度条展示）
- 🏷️ 响应式设计（支持移动端）

查看报告：

```bash
# 在浏览器中打开
open results/report.html

# 或使用绝对路径
open /path/to/results/report.html
```

## 数据集格式

数据集采用 JSON 格式，包含多个对话和对应的问题答案对：

```json
[
  {
    "conversation": {
      "speaker_a": "Alice",
      "speaker_b": "Bob",
      "session_1": [
        {"speaker": "Alice", "text": "Hello, how are you?"},
        {"speaker": "Bob", "text": "I'm fine, thanks!"}
      ],
      "session_1_date_time": "2024-01-14 10:30:00"
    },
    "qa": [
      {
        "question": "How is Bob?",
        "answer": "Bob is fine",
        "category": "1",
        "evidence": ["Bob said 'I'm fine, thanks!'"],
        "adversarial_answer": "Bob is not doing well."
      }
    ]
  }
]
```

### 问题类型 (Category)

- **Category 1**: 事实性问题 (地点、人物、事件等)
- **Category 2**: 时间性问题 (何时、时间顺序等)
- **Category 3**: 数量性问题 (多少、多长时间等)
- **Category 4**: 推理性问题 (为什么、如何等)
- **Category 5**: 复杂性问题 (需要综合多个信息)

## 高级功能

### 错误处理和重试

- 指数退避重试 (最多 3 次)
- 60 秒超时保护
- 详细的日志记录
- 失败统计追踪

### 处理统计

评估完成后会显示：

```
============================================================
📊 PROCESSING SUMMARY
============================================================
Total Conversations:      50
Successful:               48
Failed:                   2
Success Rate:             96.0%

Total Memories:           300
Successful:               298
Failed:                   2
Success Rate:             99.3%
============================================================
```

## 故障排除

### 1. Qdrant 连接失败

**错误**: 无法连接到 Qdrant 服务

**解决方案**:
```bash
# 检查 Qdrant 服务状态
curl http://localhost:6334/health

# 重启 Qdrant 服务
qdrant --host 0.0.0.0 --port 6334
```

### 2. API 调用失败

**错误**: LLM 或 Embedding API 返回错误

**解决方案**:
- 检查 `config.toml` 中的 API 密钥
- 确认 API 端点可访问
- 检查 API 额度是否充足
- 查看日志文件了解详细错误

### 3. 内存不足

**错误**: 处理大型数据集时内存溢出

**解决方案**:
- 使用较小的 `batch_size` 参数
- 减少并发请求
- 分批处理数据集

## 扩展开发

### 添加新的评估指标

在 `metrics/memory_evaluation.py` 中的 `MemorySystemEvaluator` 类中添加新方法：

```python
def calculate_custom_metric(self, ...):
    """Calculate custom evaluation metric"""
    # 实现你的指标逻辑
    return score
```

### 添加新的基线

在 `baselines/` 目录下创建新的基线模块：

```python
class NewBaseline:
    def __init__(self, config_path: str):
        # 初始化
        pass

    def answer_question(self, ...):
        # 实现基线逻辑
        pass
```

## 许可证

本项目采用 MIT 许可证。详见项目根目录的 LICENSE 文件。

## 联系信息

- 项目仓库: https://github.com/sopaco/cortex-mem
- 问题反馈: 请在 GitHub Issues 中提交

## 版本历史

### v2.0.0 (2024-12-24)
- ✨ 新增专业记忆系统评估指标 (Recall@K, MRR, NDCG)
- ✨ 新增强数据集 (50 对话, 150 问题)
- ✨ 新增统计分析功能 (置信区间, 标准差)
- ✨ 新增改进的 LLM 评判器 (0-5 分六级评分)
- ✨ 新增简单 RAG 基线对比系统
- 🛡️ 改进错误处理和重试机制
- 🧹 删除旧代码和脚本，统一评估方案
- 📄 新增 HTML 报告生成器

### v1.0.0 (2024-12-22)
- 初始版本发布
- 支持 Cortex Mem 评估
- 实现串行执行优化
- 完整的 LOCOMO 数据集支持
