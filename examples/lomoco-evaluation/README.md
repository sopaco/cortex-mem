# LOCOMO Evaluation - Cortex Mem 评估系统

## 项目概述

本项目是专为评估 Cortex Mem 记忆管理系统的性能而设计的评估框架。支持处理 LOCOMO (LoCoMo: Long-Context Conversations) 数据集，提供记忆添加、搜索和问答生成的完整评估流程。

## 核心特性

- 🎯 **专注于 Cortex Mem**: 专门针对 Cortex Mem 系统优化的评估框架
- 📊 **LOCOMO 数据集支持**: 完整支持 LOCOMO 对话数据集格式
- ⚡ **串行执行优化**: 避免并发 API 限制，兼容免费额度策略
- 🔧 **模块化设计**: 清晰的组件分离，易于扩展和维护
- 📈 **完整评估流程**: 从数据处理到结果分析的一站式解决方案

## 项目架构

```
lomoco-evaluation/
├── src/cortex_mem/           # Cortex Mem 专用模块
│   ├── add.py               # 记忆添加功能
│   ├── search.py            # 记忆搜索功能
│   └── config_utils.py      # 配置管理工具
├── dataset/                 # 数据集目录
│   ├── locomo10.json        # 主数据集文件
│   ├── locomo10_rag.json    # RAG 格式数据
│   └── locomo10_small_test.json # 小规模测试数据
├── metrics/                 # 评估指标模块
│   ├── llm_judge.py         # LLM 评判模块
│   └── utils.py             # 评估工具
├── results/                 # 结果输出目录
├── config.toml             # 主配置文件
├── run_cortex_mem_evaluation.py # 主评估脚本
├── test_cortex_mem_simple.py   # 基础测试
└── test_cortex_mem_integration.py # 集成测试
```

## 快速开始

### 1. 环境准备

确保系统已安装：
- Rust 和 Cargo
- Python 3.8+
- Qdrant 向量数据库

### 2. 启动 Qdrant 服务

```bash
# 启动 Qdrant (HTTP: 6333, gRPC: 6334)
qdrant
```

### 3. 配置 API 密钥

编辑 `config.toml` 文件，配置你的 API 密钥：

```toml
[llm]
api_base_url = "https://apis.iflow.cn/v1"
api_key = "your_iflow_api_key"
model_efficient = "qwen3-235b-a22b-instruct"

[embedding]
api_base_url = "https://ai.gitee.com/v1"
model_name = "Qwen3-Embedding-8B"
api_key = "your_gitee_embedding_api_key"
```

### 4. 运行基础测试

```bash
# 测试核心组件
python test_cortex_mem_simple.py

# 测试集成功能
python test_cortex_mem_integration.py
```

### 5. 执行评估

```bash
# 添加记忆到 Cortex Mem
python run_cortex_mem_evaluation.py --method add

# 搜索记忆并生成问答
python run_cortex_mem_evaluation.py --method search
```

## 详细使用指南

### 配置说明

#### 主要配置段

- **[qdrant]**: 向量数据库配置
- **[llm]**: 大语言模型配置 (用于生成答案)
- **[embedding]**: 嵌入模型配置 (用于向量化)
- **[memory]**: 记忆管理参数

#### 关键参数

```toml
[memory]
max_memories = 10000           # 最大记忆数量
similarity_threshold = 0.65    # 相似度阈值
max_search_results = 50        # 最大搜索结果数
enable_deduplication = true    # 启用去重
```

### 数据集格式

LOCOMO 数据集采用 JSON 格式：

```json
[
  {
    "conversation": {
      "speaker_a": "Alice",
      "speaker_b": "Bob",
      "date_time_1": "2024-01-01 10:00:00",
      "conversation_1": [
        {"speaker": "Alice", "text": "Hello, how are you?"},
        {"speaker": "Bob", "text": "I'm fine, thanks!"}
      ]
    },
    "qa": [
      {
        "question": "How is Bob?",
        "answer": "Bob is fine",
        "category": 1,
        "evidence": ["conversation_1"]
      }
    ]
  }
]
```

### API 使用模式

#### 记忆添加

```python
from cortex_mem import CortexMemAdd

# 初始化添加管理器
add_manager = CortexMemAdd(
    data_path="dataset/locomo10.json",
    batch_size=2,
    config_path="config.toml"
)

# 处理所有对话
add_manager.process_all_conversations()
```

#### 记忆搜索

```python
from cortex_mem import CortexMemSearch

# 初始化搜索管理器
search_manager = CortexMemSearch(
    output_path="results/search_results.json",
    top_k=10,
    config_path="config.toml"
)

# 处理数据并生成结果
search_manager.process_data_file("dataset/locomo10.json")
```

## 高级功能

### 自定义批处理

调整批处理大小以平衡性能和内存使用：

```python
# 小批次：更稳定，内存占用少
add_manager = CortexMemAdd(data_path="data.json", batch_size=1)

# 大批次：更快，但内存占用多
add_manager = CortexMemAdd(data_path="data.json", batch_size=5)
```

### 串行执行 vs 并发执行

当前版本使用串行执行以避免 API 并发限制：

```python
# 串行处理（当前默认）
def process_all_conversations(self):
    for idx, item in enumerate(self.data):
        self.process_conversation(item, idx)

# 如需并发处理，可修改 max_workers 参数
def process_all_conversations(self, max_workers=5):
    with ThreadPoolExecutor(max_workers=max_workers) as executor:
        # 并发处理逻辑
```

### 结果分析

生成的结果文件包含详细信息：

```json
{
  "0": [
    {
      "question": "用户问题",
      "answer": "标准答案",
      "response": "AI生成答案",
      "speaker_1_memories": [...],
      "speaker_2_memories": [...],
      "response_time": 1.23
    }
  ]
}
```

## 性能优化

### 1. 内存管理

- 使用合适的批处理大小
- 定期清理临时变量
- 监控内存使用情况

### 2. API 优化

- 串行执行避免并发限制
- 实现请求重试机制
- 使用响应缓存

### 3. 数据处理

- 使用进度条监控处理进度
- 增量处理大数据集
- 并行化独立的计算任务

## 故障排除

### 常见问题

#### 1. API 额度限制

**错误**: "免费体验访问令牌已达到最大使用额度"

**解决方案**:
- 升级到付费计划
- 使用其他兼容的 API
- 分批处理数据

#### 2. Qdrant 连接失败

**错误**: 无法连接到 Qdrant 服务

**解决方案**:
```bash
# 检查 Qdrant 服务状态
curl http://localhost:6333/health

# 重启 Qdrant 服务
qdrant --host 0.0.0.0 --port 6333
```

#### 3. 配置文件错误

**错误**: 配置文件解析失败

**解决方案**:
- 检查 TOML 语法
- 验证 API 密钥格式
- 确认路径正确性

### 调试模式

启用详细日志输出：

```python
import logging
logging.basicConfig(level=logging.DEBUG)
```

## 扩展开发

### 添加新的评估指标

1. 在 `metrics/` 目录下创建新模块
2. 实现评估函数
3. 在主评估脚本中集成

### 支持新的记忆系统

1. 创建新的 `src/<system_name>/` 目录
2. 实现 `add.py` 和 `search.py` 接口
3. 更新配置和测试脚本

### 自定义数据集格式

1. 修改数据加载逻辑
2. 更新数据验证规则
3. 添加格式转换工具

## 贡献指南

1. Fork 项目仓库
2. 创建功能分支
3. 提交变更
4. 创建 Pull Request

## 许可证

本项目采用 MIT 许可证。详见 LICENSE 文件。

## 联系信息

- 项目仓库: https://github.com/sopaco/cortex-mem
- 问题反馈: 请在 GitHub Issues 中提交

## 更新日志

### v1.0.0 (2025-12-22)
- 初始版本发布
- 支持 Cortex Mem 评估
- 实现串行执行优化
- 完整的 LOCOMO 数据集支持