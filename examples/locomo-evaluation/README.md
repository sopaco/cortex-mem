# Cortex Memory LoCoMo Evaluation

这个目录将 OpenViking / openclaw-eval 使用的 **LoCoMo 评测方法**迁移到了 `cortex-mem`，但只保留 **Cortex Memory** 单系统评测能力。

## 保留内容

- 相同的数据集形态：`locomo10.json`
- 相同的两阶段流程：`ingest` → `qa`
- 相同的 LLM judge 思路：`CORRECT / WRONG`
- 相同的 category 5 过滤策略

## 移除内容

- 任何 OpenClaw / LangMem / 其他框架对比代码
- 任何多系统 benchmark 适配代码

## 目录说明

- `eval.py`：LoCoMo ingest / qa 主脚本
- `judge.py`：使用 LLM 判定答案正确性
- `judge_util.py`：判题公共逻辑
- `stat_judge_result.py`：统计准确率、耗时、token 与分类结果
- `pyproject.toml`：Python 依赖

## 环境要求

- Python >= 3.11
- 已启动 `cortex-mem-service`
- 已配置可用的 LLM API（用于 `qa` 阶段答案生成，以及 `judge` 阶段判题）

## 安装依赖

```bash
cd examples/locomo-evaluation
python -m venv .venv
source .venv/bin/activate
pip install -e .
```

## 数据集

本工具不内置 `locomo10.json`，你需要自行准备与 OpenViking 相同格式的 LoCoMo 数据文件。

期望结构：

```text
locomo10.json -> list of samples
  sample
    ├── sample_id
    ├── conversation
    │   ├── speaker_a / speaker_b
    │   ├── session_N_date_time
    │   └── session_N
    │       └── { speaker, dia_id, text, img_url?, blip_caption?, query? }
    ├── qa
    │   └── { question, answer, evidence, category }
    ├── event_summary
    ├── observation
    └── session_summary
```

## 运行前准备

先启动 Cortex 服务，例如：

```bash
cargo run -p cortex-mem-service -- --port 8085 --data-dir ./cortex-data
```

如果 `qa` 阶段需要基于检索结果再生成最终答案，请配置：

```bash
export OPENAI_BASE_URL="https://api.openai.com/v1"
export OPENAI_API_KEY="your-key"
export EVAL_ANSWER_MODEL="gpt-5-mini"
```

`judge.py` 默认也会读取 `OPENAI_BASE_URL` / `OPENAI_API_KEY`。

## 评测流程

### 1. ingest

将 LoCoMo 对话按 session 打包写入 Cortex Memory。每个 sample 默认使用独立 tenant，避免样本之间相互污染。

```bash
python eval.py ingest ./locomo10.json --sample 0 --sessions 1-4 --output ./output/ingest.txt
```

可选参数：

- `--base-url`：Cortex service 地址，默认 `http://127.0.0.1:8085`
- `--tenant`：手动指定 tenant
- `--tenant-prefix`：自动生成 tenant 前缀，默认 `locomo-eval`
- `--user`：手动指定 user id
- `--tail`：每个 session 末尾附加文本，默认 `[]`
- `--wait-timeout`：ingest 后等待记忆可搜索的超时秒数

### 2. qa

对已 ingest 的 sample 运行 QA。流程为：

1. 在对应 tenant 中搜索相关记忆
2. 用检索到的上下文生成答案
3. 输出问题、标准答案、模型回答、上下文、耗时、token

```bash
python eval.py qa ./locomo10.json --sample 0 --output ./output/qa_results.txt --top-k 5 --count 20
```

输出文件：

- `qa_results.txt`：总体 token 摘要
- `qa_results.txt.json`：完整回答记录
- `qa_results.txt.<sample_idx>.jsonl`：逐 sample 明细

### 3. judge

```bash
python judge.py ./output/qa_results.txt.json --output ./output/grades.json --model gpt-5-mini
```

### 4. 统计

```bash
python stat_judge_result.py --input ./output/grades.json
```

会生成：

- 控制台统计输出
- `summary.txt`

## 方法差异说明

虽然数据集与评测流程保持 OpenViking / openclaw-eval 一致，但这里的执行对象改为 **Cortex Memory**：

- `ingest` 不再调用 OpenClaw 或 `ov add-memory`
- 改为直接调用 `cortex-mem-service` 的：
  - `/api/v2/tenants/switch`
  - `/api/v2/sessions`
  - `/api/v2/sessions/:thread_id/messages`
  - `/api/v2/sessions/:thread_id/close`
  - `/api/v2/search/`

因此，这个工具现在是 **Cortex Memory 专用评测工具**，适合作为后续架构优化前后的统一基线。
