# Cortex Mem 评估集成指南

## 概述

本指南介绍如何在 mem0 评估框架中集成 Cortex Mem 内存系统的评估。Cortex Mem 是一个基于 Rust 的 AI 代理内存管理系统，本扩展使其能够与 mem0、Zep、LangMem 等其他内存技术进行公平比较。

## 架构设计

### 扩展结构
```
对标项目的源码-mem0/evaluation/
├── src/
│   ├── cortex_mem/          # Cortex Mem 评估模块
│   │   ├── __init__.py
│   │   ├── add.py          # 记忆添加实现
│   │   ├── search.py       # 记忆搜索实现
│   │   └── config_utils.py # 配置工具
│   ├── utils.py            # 已扩展支持 cortex_mem
│   └── ...
├── config.toml             # Cortex Mem 配置文件
├── .env.example           # 环境变量示例
└── run_experiments.py     # 已扩展支持 cortex_mem
```

### 技术集成点
1. **Python-Rust 桥接**: 通过子进程调用 `cortex-mem-cli` 二进制文件
2. **配置管理**: 使用 `config.toml` 中的 OpenAI 配置作为 provider
3. **环境变量**: 支持 `${VAR_NAME}` 语法在配置文件中
4. **评估指标**: 复用 mem0 的 BLEU、F1、LLM 评分体系

## 快速开始

### 1. 环境设置

```bash
# 进入评估目录
cd 对标项目的源码-mem0/evaluation

# 复制环境变量文件
cp .env.example .env

# 编辑 .env 文件，设置你的 API 密钥
# 主要需要设置 OPENAI_API_KEY
```

### 2. 配置检查

确保 `config.toml` 文件存在并正确配置。主要配置项：

```toml
[llm]
api_key = "${OPENAI_API_KEY}"  # 从环境变量读取
model_efficient = "gpt-3.5-turbo"

[embedding]
api_key = "${OPENAI_API_KEY}"  # 从环境变量读取
model_name = "text-embedding-ada-002"

[qdrant]
url = "http://localhost:6333"  # 需要运行 Qdrant
```

### 3. 构建 Cortex Mem

```bash
# 在项目根目录构建 CLI
cd ../..
cargo build --bin cortex-mem-cli
```

### 4. 运行评估

#### 添加记忆阶段
```bash
python run_experiments.py --technique_type cortex_mem --method add
```

#### 搜索记忆阶段
```bash
python run_experiments.py --technique_type cortex_mem --method search
```

#### 完整评估流程
```bash
# 1. 添加所有对话记忆
python run_experiments.py --technique_type cortex_mem --method add

# 2. 搜索并回答问题
python run_experiments.py --technique_type cortex_mem --method search --top_k 10

# 3. 运行评估
python evals.py --input_file results/cortex_mem_results.json --output_file results/cortex_mem_evaluated.json

# 4. 生成分数
python generate_scores.py
```

## 配置详解

### config.toml 结构

```toml
# LLM 配置 - 使用 OpenAI
[llm]
api_base_url = "https://api.openai.com/v1"
api_key = "${OPENAI_API_KEY}"  # 环境变量替换
model_efficient = "gpt-3.5-turbo"
temperature = 0.0
max_tokens = 4096

# 嵌入模型配置
[embedding]
api_base_url = "https://api.openai.com/v1"
model_name = "text-embedding-ada-002"
api_key = "${OPENAI_API_KEY}"  # 环境变量替换
batch_size = 32

# Qdrant 向量数据库
[qdrant]
url = "http://localhost:6333"
collection_name = "cortex_mem_evaluation"
embedding_dim = 1536  # text-embedding-ada-002 维度

# 内存管理器配置
[memory]
similarity_threshold = 0.65
max_search_results = 50
auto_enhance = true
deduplicate = true
```

### 环境变量支持

配置文件支持 `${VAR_NAME}` 语法，会自动从环境变量替换：

```toml
api_key = "${OPENAI_API_KEY}"  # 替换为环境变量值
```

## 实现细节

### 1. 记忆添加 (add.py)
- 使用 `cortex-mem-cli add` 命令添加记忆
- 批量处理对话数据
- 为每个说话者创建独立的用户 ID
- 包含时间戳和话题标签

### 2. 记忆搜索 (search.py)
- 使用 `cortex-mem-cli search` 命令搜索记忆
- 支持 top_k 参数控制返回结果数量
- 集成 OpenAI 进行答案生成
- 记录搜索延迟和 token 消耗

### 3. 配置管理 (config_utils.py)
- 环境变量替换
- 临时配置文件生成
- 配置验证
- 自动清理临时文件

## 与其他技术的比较

### 优势
1. **可对比性**: 使用相同的 LOCOMO 数据集和评估指标
2. **配置灵活**: 支持环境变量和配置文件
3. **Rust 性能**: 底层使用 Rust 实现，性能优秀
4. **完整集成**: 支持 add/search 完整评估流程

### 限制
1. **CLI 依赖**: 需要通过子进程调用二进制文件
2. **输出解析**: 需要解析 CLI 的文本输出
3. **Qdrant 依赖**: 需要运行 Qdrant 向量数据库

## 故障排除

### 常见问题

1. **找不到 config.toml**
   ```
   FileNotFoundError: Could not find config.toml file
   ```
   解决方案：确保 `config.toml` 在评估目录或父目录中

2. **OpenAI API 密钥未设置**
   ```
   ValueError: OPENAI_API_KEY environment variable is not set
   ```
   解决方案：在 `.env` 文件中设置 `OPENAI_API_KEY`

3. **Qdrant 连接失败**
   ```
   Error: 无法连接到 Qdrant
   ```
   解决方案：启动 Qdrant 服务：`docker run -p 6333:6333 qdrant/qdrant`

4. **CLI 构建失败**
   ```
   error: could not find `cortex-mem-cli` in package `cortex-mem`
   ```
   解决方案：在项目根目录运行 `cargo build --bin cortex-mem-cli`

### 调试模式

启用详细日志：
```python
# 在代码中添加
import logging
logging.basicConfig(level=logging.DEBUG)
```

## 扩展开发

### 添加新功能

1. **支持更多参数**:
   ```python
   # 在 search.py 中添加
   def search_memory(self, user_id, query, filters=None, **kwargs):
       # 支持更多搜索参数
       pass
   ```

2. **优化输出解析**:
   ```python
   # 改进 CLI 输出解析
   def parse_cli_output(self, stdout):
       # 实现更健壮的解析逻辑
       pass
   ```

3. **添加缓存机制**:
   ```python
   # 减少重复 API 调用
   import functools
   @functools.lru_cache(maxsize=128)
   def search_with_cache(self, query):
       pass
   ```

### 性能优化建议

1. **并行处理**: 使用 `ThreadPoolExecutor` 加速批量操作
2. **连接池**: 复用 CLI 进程连接
3. **结果缓存**: 缓存频繁查询的结果
4. **增量评估**: 支持从断点继续评估

## 贡献指南

1. 遵循现有代码风格和结构
2. 添加适当的错误处理和日志
3. 更新文档和示例
4. 添加单元测试
5. 确保向后兼容性

## 许可证

本项目基于 MIT 许可证。详见 LICENSE 文件。