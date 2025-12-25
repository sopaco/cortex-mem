# 新设备快速上手指南

## 📋 前提条件

### 硬件要求
- CPU: 4核以上推荐
- 内存: 8GB+ (数据集越大需求越高)
- 磁盘: 10GB+ 可用空间

### 软件要求
- **macOS**: 10.15+ 或任意 Linux 发行版
- **Python**: 3.8 或更高版本
- **Rust**: 1.70+ (用于编译 cortex-mem)
- **Qdrant**: 1.7+ 向量数据库

---

## 🚀 快速开始（5分钟）

### 步骤1: 克隆项目

```bash
# 进入工作目录
cd ~/workspace

# 克隆 cortex-mem 仓库
git clone https://github.com/sopaco/cortex-mem.git

# 进入评估目录
cd cortex-mem/examples/lomoco-evaluation
```

### 步骤2: 安装 Python 依赖

```bash
# 进入评估目录
cd ~/workspace/cortex-mem/examples/lomoco-evaluation

# 创建虚拟环境（推荐）
python3 -m venv venv
source venv/bin/activate  # macOS/Linux
# Windows: venv\Scripts\activate

# 安装依赖
pip install --upgrade pip
pip install openai httpx toml tqdm jinja2 sentence-transformers scipy numpy
```

### 步骤3: 配置 API 密钥

```bash
# 复制示例配置
cp .env.example .env

# 编辑 .env 文件
vim .env  # 或使用其他编辑器
```

在 `.env` 中配置你的 API：

```bash
# LLM 配置
LLM_API_BASE_URL="https://your-llm-api.com/v1"
LLM_API_KEY="your_api_key_here"
LLM_MODEL="your_model_name"

# Embedding 配置
EMBEDDING_API_BASE_URL="https://your-embedding-api.com/v1"
EMBEDDING_API_KEY="your_api_key_here"
EMBEDDING_MODEL="your_embedding_model"

# Qdrant 配置
QDRANT_URL="http://localhost:6334"
```

**或者直接编辑 `config.toml`**:

```bash
# 编辑 config.toml
vim config.toml
```

### 步骤4: 启动 Qdrant

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

# 或使用 Docker
docker run -d -p 6333:6333 -p 6334:6334 \
  -v $(pwd)/qdrant_storage:/qdrant/storage \
  qdrant/qdrant
```

验证 Qdrant 运行状态:

```bash
# 检查健康状态
curl http://localhost:6334/health

# 应该返回: {"status":"ok"}
```

### 步骤5: 编译 Cortex Mem

```bash
# 返回到 cortex-mem 根目录
cd ~/workspace/cortex-mem

# 编译 release 版本
cargo build --release -p cortex-mem-cli

# 验证编译成功
ls -lh target/release/cortex-mem-cli
```

### 步骤6: 运行测试

```bash
# 返回评估目录
cd examples/lomoco-evaluation

# 运行小型测试（使用 locomo10.json）
python run_cortex_mem_evaluation.py --method add --data dataset/locomo10.json
python run_cortex_mem_evaluation.py --method search --data dataset/locomo10.json
```

---

## 📊 完整评估流程

### 1. 使用小型数据集测试

```bash
cd ~/workspace/cortex-mem/examples/lomoco-evaluation

# 添加记忆（约 2-5 分钟）
python run_cortex_mem_evaluation.py --method add --data dataset/locomo10.json

# 搜索记忆（约 3-10 分钟）
python run_cortex_mem_evaluation.py --method search --data dataset/locomo10.json --top_k 10

# 评估结果
python -m metrics.memory_evaluation \
  --results results/cortex_mem_results.json \
  --dataset dataset/locomo10.json \
  --output results/cortex_mem_test_eval.json
```

**预期输出**:
```
============================================================
📊 PROCESSING SUMMARY
============================================================
Total Conversations:      10
Successful:               10
Failed:                   0
Success Rate:             100.0%
============================================================
```

### 2. 使用大型数据集正式评估

```bash
# 添加记忆（约 10-30 分钟）
python run_cortex_mem_evaluation.py --method add --data dataset/locomo50.json

# 搜索记忆（约 30-90 分钟）
python run_cortex_mem_evaluation.py --method search --data dataset/locomo50.json --top_k 10

# 评估结果
python -m metrics.memory_evaluation \
  --results results/cortex_mem_results.json \
  --dataset dataset/locomo50.json \
  --output results/cortex_mem_final_eval.json
```

### 3. 运行基线对比

```bash
# 简单 RAG 基线（约 20-60 分钟）
python baselines/simple_rag.py \
  --data dataset/locomo50.json \
  --output results/simple_rag_results.json

# 评估基线结果
python -m metrics.memory_evaluation \
  --results results/simple_rag_results.json \
  --dataset dataset/locomo50.json \
  --output results/simple_rag_eval.json
```

---

## 🛠️ 高级配置

### 自定义数据集

```bash
# 生成 100 个对话的数据集
python generate_enhanced_dataset.py

# 编辑脚本调整参数
vim generate_enhanced_dataset.py

# 修改第 180 行:
# return generate_enhanced_dataset(num_conversations=100)
```

### 调整批处理大小

```bash
# 减小 batch_size 以降低内存使用
python run_cortex_mem_evaluation.py \
  --method add \
  --data dataset/locomo50.json \
  --batch_size 1

# 增大 batch_size 以提高速度（需要更多内存）
python run_cortex_mem_evaluation.py \
  --method add \
  --data dataset/locomo50.json \
  --batch_size 5
```

### 调整检索数量

```bash
# 检索更多记忆
python run_cortex_mem_evaluation.py \
  --method search \
  --data dataset/locomo50.json \
  --top_k 20

# 检索更少记忆（更快但可能遗漏信息）
python run_cortex_mem_evaluation.py \
  --method search \
  --data dataset/locomo50.json \
  --top_k 5
```

---

## 🔧 故障排除

### 问题1: Qdrant 连接失败

**症状**: `Failed to connect to Qdrant`

**解决方案**:
```bash
# 检查 Qdrant 是否运行
curl http://localhost:6334/health

# 检查端口是否被占用
lsof -i :6334  # macOS/Linux
netstat -an | grep 6334  # Linux

# 重启 Qdrant
pkill qdrant
qdrant --host 0.0.0.0 --port 6334
```

### 问题2: Python 模块导入失败

**症状**: `ModuleNotFoundError: No module named 'xxx'`

**解决方案**:
```bash
# 确保在正确的目录
cd ~/workspace/cortex-mem/examples/lomoco-evaluation

# 重新安装依赖
pip install --upgrade openai httpx toml tqdm jinja2 sentence-transformers scipy numpy

# 使用虚拟环境
source venv/bin/activate
```

### 问题3: API 调用失败

**症状**: `Failed to add memory: ... API error ...`

**解决方案**:
```bash
# 检查 API 密钥
cat .env | grep API_KEY

# 测试 API 连接
curl -X POST "https://your-api.com/v1/chat/completions" \
  -H "Authorization: Bearer YOUR_KEY" \
  -H "Content-Type: application/json" \
  -d '{"model":"test","messages":[{"role":"user","content":"Hi"}]}'

# 检查 API 配额
# 登录 API 提供商控制台查看配额
```

### 问题4: 内存不足

**症状**: `MemoryError` 或 `Killed: 9`

**解决方案**:
```bash
# 使用小数据集
python run_cortex_mem_evaluation.py --method add --data dataset/locomo10.json

# 减小 batch_size
python run_cortex_mem_evaluation.py --method add --batch_size 1

# 增加 swap 空间（Linux）
sudo fallocate -l 4G /swapfile
sudo chmod 600 /swapfile
sudo mkswap /swapfile
sudo swapon /swapfile
```

---

## 📝 预期时间和资源使用

### 数据集规模对比

| 数据集 | 对话数 | 问题数 | 添加时间 | 搜索时间 | 内存使用 |
|--------|--------|--------|----------|----------|----------|
| locomo10.json | 10 | ~30 | 2-5 分钟 | 3-10 分钟 | ~2 GB |
| locomo50.json | 50 | 150 | 10-30 分钟 | 30-90 分钟 | ~4-8 GB |

### 性能优化建议

1. **使用 GPU**: 如果可用，GPU 可加速 embedding 计算
2. **增量处理**: 分批处理大型数据集
3. **缓存结果**: Qdrant 会缓存向量，重复查询更快
4. **并发控制**: 不要同时运行多个评估实例

---

## ✅ 验证安装成功

运行以下命令验证所有组件：

```bash
# 检查 Python 版本
python --version  # 应该 >= 3.8

# 检查依赖安装
python -c "import openai, httpx, toml, tqdm, jinja2, sentence_transformers, scipy, numpy; print('✅ All dependencies OK')"

# 检查 Qdrant
curl -s http://localhost:6334/health | grep ok && echo '✅ Qdrant OK'

# 检查 Rust
cargo --version  # 应该 >= 1.70

# 运行测试评估
python run_cortex_mem_evaluation.py --method add --data dataset/locomo10.json
```

如果所有检查都显示 ✅，说明安装成功！

---

## 📚 进一步学习

- **README.md**: 完整的评估框架说明
- **CLEANUP_REPORT.md**: 清理后的目录结构说明
- **config.toml**: 详细的配置选项注释

---

## 🆘 需要帮助？

遇到问题？

1. 查看日志文件: `logs/cortex-mem-*.log`
2. 启用调试模式: 在代码中设置 `logging.basicConfig(level=logging.DEBUG)`
3. 提交 Issue: https://github.com/sopaco/cortex-mem/issues

---

*快速上手版本: 1.0.0*
*最后更新: 2024-12-24*
