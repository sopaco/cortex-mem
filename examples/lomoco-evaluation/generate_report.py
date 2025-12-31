#!/usr/bin/env python3
"""
评估报告生成器 - 将评估结果转换为HTML格式
"""

import argparse
import json
import os
from datetime import datetime
from pathlib import Path


def format_value(value, precision=4):
    """格式化数值，添加千分位分隔符"""
    if isinstance(value, (int, float)):
        if value >= 1:
            return f"{value:,.{precision}f}".replace(",", " ")
        return f"{value:.{precision}f}"
    return str(value)


def get_rating_color(score):
    """根据分数返回颜色"""
    if score >= 0.9:
        return "#10b981"  # 绿色
    elif score >= 0.7:
        return "#f59e0b"  # 黄色
    else:
        return "#ef4444"  # 红色


def get_rating_label(score):
    """根据分数返回评级标签"""
    if score >= 0.9:
        return "优秀"
    elif score >= 0.7:
        return "良好"
    else:
        return "需要改进"


def generate_html(results_file, output_file="report.html"):
    """生成HTML报告"""

    # 读取评估结果
    with open(results_file, "r", encoding="utf-8") as f:
        data = json.load(f)

    overall = data.get("overall", {})
    categories = {k: v for k, v in data.items() if k.startswith("category_")}

    # 根据文件名确定系统名称
    system_name = "记忆系统"
    if "cortex_mem" in results_file.lower():
        system_name = "Cortex Memory"
    elif "langmem" in results_file.lower():
        system_name = "LangMem"
    elif "simple_rag" in results_file.lower():
        system_name = "Simple RAG"

    # 指标定义
    metrics_info = {
        "recall_at_1": {
            "name": "Recall@1",
            "category": "检索质量",
            "description": "第一条检索结果中至少包含一个相关记忆的概率",
        },
        "recall_at_3": {
            "name": "Recall@3",
            "category": "检索质量",
            "description": "前3条检索结果中至少包含一个相关记忆的概率",
        },
        "recall_at_5": {
            "name": "Recall@5",
            "category": "检索质量",
            "description": "前5条检索结果中至少包含一个相关记忆的概率",
        },
        "recall_at_10": {
            "name": "Recall@10",
            "category": "检索质量",
            "description": "前10条检索结果中至少包含一个相关记忆的概率",
        },
        "precision_at_1": {
            "name": "Precision@1",
            "category": "检索质量",
            "description": "第一条检索结果中相关记忆的比例",
        },
        "precision_at_3": {
            "name": "Precision@3",
            "category": "检索质量",
            "description": "前3条检索结果中相关记忆的比例",
        },
        "precision_at_5": {
            "name": "Precision@5",
            "category": "检索质量",
            "description": "前5条检索结果中相关记忆的比例",
        },
        "mrr": {
            "name": "MRR",
            "category": "排名质量",
            "description": "第一个相关记忆排名的倒数平均值（1.0表示相关记忆在第一位）",
        },
        "ndcg_at_5": {
            "name": "NDCG@5",
            "category": "排名质量",
            "description": "归一化折损累计增益，综合考量排名位置和相关性的指标",
        },
        "ndcg_at_10": {
            "name": "NDCG@10",
            "category": "排名质量",
            "description": "归一化折损累计增益，综合考量排名位置和相关性的指标（前10条）",
        },
        "answer_semantic_similarity": {
            "name": "语义相似度",
            "category": "答案质量",
            "description": "生成答案与标准答案的语义相似程度（使用Sentence BERT计算）",
        },
        "answer_exact_match": {
            "name": "精确匹配",
            "category": "答案质量",
            "description": "生成答案与标准答案完全一致的比例",
        },
        "answer_keyword_f1": {
            "name": "关键词 F1",
            "category": "答案质量",
            "description": "基于关键词重叠的 F1 分数",
        },
    }

    # 生成HTML
    html_content = f"""<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{system_name} 评估报告</title>
    <style>
        * {{
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }}

        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
            background: #1a1a2e;
            padding: 20px;
            min-height: 100vh;
        }}

        .container {{
            max-width: 1400px;
            margin: 0 auto;
            background: white;
            border-radius: 2px;
            box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
            overflow: hidden;
        }}

        .header {{
            background: #1e3a5f;
            color: white;
            padding: 20px 30px;
            text-align: center;
        }}

        .header h1 {{
            font-size: 1.8em;
            margin-bottom: 5px;
            font-weight: 700;
        }}

        .header .subtitle {{
            font-size: 0.9em;
            opacity: 0.9;
        }}

        .header .date {{
            margin-top: 8px;
            font-size: 0.8em;
            opacity: 0.8;
        }}

        .summary {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 15px;
            padding: 25px;
            background: #f8fafc;
        }}

        .summary-card {{
            background: white;
            padding: 15px;
            border-radius: 2px;
            box-shadow: 0 1px 3px rgba(0, 0, 0, 0.08);
            text-align: center;
            transition: transform 0.2s ease;
        }}

        .summary-card:hover {{
            transform: translateY(-1px);
        }}

        .summary-card h3 {{
            color: #4a5568;
            font-size: 0.8em;
            text-transform: uppercase;
            letter-spacing: 0.5px;
            margin-bottom: 8px;
        }}

        .summary-card .value {{
            font-size: 1.8em;
            font-weight: 700;
            color: #1e3a5f;
            margin-bottom: 3px;
        }}

        .summary-card .label {{
            color: #94a3b8;
            font-size: 0.75em;
        }}

        .content {{
            padding: 25px;
        }}

        .section {{
            margin-bottom: 30px;
        }}

        .section h2 {{
            color: #1e293b;
            font-size: 1.4em;
            margin-bottom: 15px;
            padding-bottom: 8px;
            border-bottom: 2px solid #1e3a5f;
        }}

        .card-grid {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
            gap: 15px;
            margin-bottom: 25px;
        }}

        .card {{
            background: white;
            padding: 15px;
            border-radius: 2px;
            box-shadow: 0 1px 3px rgba(0, 0, 0, 0.08);
            border-left: 3px solid #1e3a5f;
        }}

        .card:hover {{
            box-shadow: 0 2px 6px rgba(0, 0, 0, 0.12);
        }}

        .metric-name {{
            font-size: 1em;
            font-weight: 600;
            color: #1e293b;
            margin-bottom: 10px;
        }}

        .metric-value {{
            font-size: 2em;
            font-weight: 700;
            color: #1e3a5f;
            margin: 10px 0;
        }}

        .metric-details {{
            color: #64748b;
            font-size: 0.85em;
            line-height: 1.5;
        }}

        .metric-details > div {{
            margin-bottom: 4px;
        }}

        .rating-badge {{
            display: inline-block;
            padding: 4px 10px;
            border-radius: 2px;
            font-size: 0.8em;
            font-weight: 600;
            color: white;
            margin-top: 10px;
        }}

        .table {{
            width: 100%;
            border-collapse: collapse;
            background: white;
            border-radius: 2px;
            overflow: hidden;
            box-shadow: 0 1px 3px rgba(0, 0, 0, 0.08);
        }}

        .table thead {{
            background: #1e3a5f;
            color: white;
        }}

        .table th {{
            padding: 10px;
            text-align: left;
            font-weight: 600;
            font-size: 0.85em;
        }}

        .table td {{
            padding: 10px;
            border-bottom: 1px solid #e2e8f0;
            font-size: 0.9em;
        }}

        .table tbody tr:hover {{
            background: #f8fafc;
        }}

        .info-grid {{
            display: grid;
            grid-template-columns: 1fr 1fr;
            gap: 10px;
            margin-top: 10px;
        }}

        .info-item {{
            padding: 8px 10px;
            background: #f8fafc;
            border-radius: 2px;
        }}

        .info-label {{
            font-size: 0.75em;
            color: #64748b;
            font-weight: 500;
        }}

        .info-value {{
            font-size: 1em;
            font-weight: 600;
            color: #1e3a5f;
        }}

        .legend {{
            background: #f0f4f8;
            padding: 15px;
            border-radius: 2px;
            margin-top: 20px;
            border-left: 3px solid #1e3a5f;
            font-size: 0.85em;
        }}

        .legend h3 {{
            color: #1e3a5f;
            margin-bottom: 12px;
            font-size: 1em;
        }}

        .legend-item {{
            display: flex;
            align-items: center;
            margin-bottom: 6px;
        }}

        .legend-color {{
            width: 16px;
            height: 16px;
            border-radius: 2px;
            margin-right: 8px;
            flex-shrink: 0;
        }}

        .footer {{
            text-align: center;
            padding: 15px;
            color: #64748b;
            font-size: 0.8em;
            background: #f8fafc;
            border-top: 1px solid #e2e8f0;
        }}

        @media (max-width: 768px) {{
            .summary {{
                grid-template-columns: 1fr;
            }}

            .card-grid {{
                grid-template-columns: 1fr;
            }}

            .table {{
                font-size: 0.85em;
            }}

            .table th,
            .table td {{
                padding: 8px;
            }}
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>📊 {system_name} 评估报告</h1>
            <p class="subtitle">记忆系统性能评估报告</p>
            <p class="date">生成时间: {datetime.now().strftime("%Y-%m-%d %H:%M:%S")}</p>
        </div>

        <!-- 总体指标概览 -->
        <div class="summary">
"""

    # 生成核心指标卡片
    key_metrics = [
        ("recall_at_1", "检索覆盖率 @1"),
        ("precision_at_1", "检索精确度 @1"),
        ("mrr", "排名质量"),
        ("answer_semantic_similarity", "答案语义相似度"),
        ("answer_exact_match", "精确匹配率"),
    ]

    for metric_key, label in key_metrics:
        if metric_key in overall:
            metric_data = overall[metric_key]
            html_content += f"""
            <div class="summary-card">
                <h3>{label}</h3>
                <div class="value">{format_value(metric_data["mean"], 3)}</div>
                <div class="label">标准差: ±{format_value(metric_data["std"], 3)}</div>
            </div>
"""

    html_content += """
        </div>
"""

    # 指标对比表格
    html_content += """
        <div class="content">
            <div class="section">
                <h2>📊 指标对比表格</h2>

                <table class="table">
                    <thead>
                        <tr>
                            <th>指标名称</th>
                            <th>类别</th>
                            <th>均值</th>
                            <th>标准差</th>
                            <th>95% 置信区间</th>
                            <th>样本数</th>
                            <th>评级</th>
                        </tr>
                    </thead>
                    <tbody>
"""

    for metric_key in sorted(overall.keys()):
        if metric_key in metrics_info:
            metric_data = overall[metric_key]
            info = metrics_info[metric_key]
            rating = get_rating_label(metric_data["mean"])

            ci_low, ci_high = metric_data["confidence_interval_95"]

            html_content += f"""
                        <tr>
                            <td><strong>{info["name"]}</strong></td>
                            <td>{info["category"]}</td>
                            <td>{format_value(metric_data["mean"], 4)}</td>
                            <td>{format_value(metric_data["std"], 4)}</td>
                            <td>{format_value(ci_low, 4)} - {format_value(ci_high, 4)}</td>
                            <td>{metric_data["count"]}</td>
                            <td style="color: {get_rating_color(metric_data["mean"])}; font-weight: 600;">{rating}</td>
                        </tr>
"""

    html_content += """
                    </tbody>
                </table>
            </div>
"""

    # 按分类别的指标
    html_content += """
            <div class="section">
                <h2>📂 分类指标详情</h2>

                <div class="card-grid">
"""

    category_names = {
        "category_1": "事实性问题",
        "category_2": "时间性问题",
        "category_3": "数量性问题",
    }

    for cat_key, cat_name in category_names.items():
        if cat_key in categories:
            cat_data = categories[cat_key]
            html_content += f"""
                    <div class="card">
                        <h3 style="margin: 0 0 12px 0; color: #1e293b; font-size: 1em;">{cat_name}</h3>
                        <div class="info-grid">
                            <div class="info-item">
                                <div class="info-label">问题数量</div>
                                <div class="info-value">{cat_data.get("recall_at_1", {}).get("count", 0)}</div>
                            </div>
                            <div class="info-item">
                                <div class="info-label">Recall@1</div>
                                <div class="info-value">{format_value(cat_data.get("recall_at_1", {}).get("mean", 0), 3)}</div>
                            </div>
                            <div class="info-item">
                                <div class="info-label">Precision@1</div>
                                <div class="info-value">{format_value(cat_data.get("precision_at_1", {}).get("mean", 0), 3)}</div>
                            </div>
                            <div class="info-item">
                                <div class="info-label">MRR</div>
                                <div class="info-value">{format_value(cat_data.get("mrr", {}).get("mean", 0), 3)}</div>
                            </div>
                            <div class="info-item">
                                <div class="info-label">语义相似度</div>
                                <div class="info-value">{format_value(cat_data.get("answer_semantic_similarity", {}).get("mean", 0), 3)}</div>
                            </div>
                        </div>
                    </div>
"""

    html_content += """
                </div>
            </div>
"""

    # 指标说明
    html_content += """
            <div class="section">
                <h2>📖 指标定义和说明</h2>

                <div class="card-grid">
"""

    for metric_key, info in metrics_info.items():
        if metric_key in overall:
            html_content += f"""
                    <div class="card">
                        <div class="metric-name">{info["name"]}</div>
                        <div class="metric-details">
                            <div style="margin-bottom: 6px;"><strong>类别:</strong> {info["category"]}</div>
                            <div><strong>说明:</strong> {info["description"]}</div>
                        </div>
                    </div>
"""

    html_content += """
                </div>
            </div>

            <div class="legend">
                <h3>📊 评级说明</h3>
                <div class="legend-item">
                    <div class="legend-color" style="background: #10b981;"></div>
                    <span>优秀 (≥ 0.90)</span>
                </div>
                <div class="legend-item">
                    <div class="legend-color" style="background: #f59e0b;"></div>
                    <span>良好 (0.70 - 0.89)</span>
                </div>
                <div class="legend-item">
                    <div class="legend-color" style="background: #ef4444;"></div>
                    <span>需要改进 (&lt; 0.70)</span>
                </div>
            </div>
        </div>
"""

    # 页脚
    html_content += f"""
        <div class="footer">
            <p>{system_name} 评估系统 v2.0</p>
            <p>生成时间: {datetime.now().strftime("%Y-%m-%d %H:%M:%S")}</p>
        </div>
    </div>
</body>
</html>
"""

    # 保存HTML文件
    with open(output_file, "w", encoding="utf-8") as f:
        f.write(html_content)

    print(f"✅ HTML报告已生成: {output_file}")
    return output_file


def main():
    parser = argparse.ArgumentParser(description="生成评估报告HTML")
    parser.add_argument(
        "--results",
        type=str,
        default="results/cortex_mem_evaluated.json",
        help="评估结果文件路径",
    )
    parser.add_argument(
        "--output", type=str, default="report.html", help="输出的HTML文件路径"
    )

    args = parser.parse_args()

    # 生成HTML报告
    generate_html(args.results, args.output)

    print(f"\n📋 报告路径: {os.path.abspath(args.output)}")
    print("💡 在浏览器中打开: open " + os.path.abspath(args.output))


if __name__ == "__main__":
    main()
