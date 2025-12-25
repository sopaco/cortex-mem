#!/usr/bin/env python3
"""
评估报告生成器 - 将评估结果转换为HTML格式
"""

import json
import argparse
import os
from pathlib import Path
from datetime import datetime


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
    with open(results_file, 'r', encoding='utf-8') as f:
        data = json.load(f)
    
    overall = data.get('overall', {})
    categories = {k: v for k, v in data.items() if k.startswith('category_')}
    
    # 指标定义
    metrics_info = {
        'recall_at_1': {
            'name': 'Recall@1',
            'category': '检索质量',
            'description': '第一条检索结果中至少包含一个相关记忆的概率'
        },
        'recall_at_3': {
            'name': 'Recall@3',
            'category': '检索质量',
            'description': '前3条检索结果中至少包含一个相关记忆的概率'
        },
        'recall_at_5': {
            'name': 'Recall@5',
            'category': '检索质量',
            'description': '前5条检索结果中至少包含一个相关记忆的概率'
        },
        'recall_at_10': {
            'name': 'Recall@10',
            'category': '检索质量',
            'description': '前10条检索结果中至少包含一个相关记忆的概率'
        },
        'precision_at_1': {
            'name': 'Precision@1',
            'category': '检索质量',
            'description': '第一条检索结果中相关记忆的比例'
        },
        'precision_at_3': {
            'name': 'Precision@3',
            'category': '检索质量',
            'description': '前3条检索结果中相关记忆的比例'
        },
        'precision_at_5': {
            'name': 'Precision@5',
            'category': '检索质量',
            'description': '前5条检索结果中相关记忆的比例'
        },
        'mrr': {
            'name': 'MRR',
            'category': '排名质量',
            'description': '第一个相关记忆排名的倒数平均值（1.0表示相关记忆在第一位）'
        },
        'ndcg_at_5': {
            'name': 'NDCG@5',
            'category': '排名质量',
            'description': '归一化折损累计增益，综合考量排名位置和相关性的指标'
        },
        'ndcg_at_10': {
            'name': 'NDCG@10',
            'category': '排名质量',
            'description': '归一化折损累计增益，综合考量排名位置和相关性的指标（前10条）'
        },
        'answer_semantic_similarity': {
            'name': '语义相似度',
            'category': '答案质量',
            'description': '生成答案与标准答案的语义相似程度（使用Sentence BERT计算）'
        },
        'answer_exact_match': {
            'name': '精确匹配',
            'category': '答案质量',
            'description': '生成答案与标准答案完全一致的比例'
        },
        'answer_keyword_f1': {
            'name': '关键词 F1',
            'category': '答案质量',
            'description': '基于关键词重叠的 F1 分数'
        }
    }
    
    # 生成HTML
    html_content = f"""<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Cortex Mem 评估报告</title>
    <style>
        * {{
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }}
        
        body {{
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
            line-height: 1.6;
            color: #333;
            background: #f5f5f5;
            padding: 20px;
        }}
        
        .container {{
            max-width: 1400px;
            margin: 0 auto;
        }}
        
        .header {{
            text-align: center;
            margin-bottom: 40px;
            padding: 30px;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            border-radius: 10px;
            color: white;
        }}
        
        .header h1 {{
            margin: 0 0 10px 0;
            font-size: 2.5em;
        }}
        
        .header p {{
            margin: 10px 0 0 0;
            font-size: 1.1em;
            opacity: 0.9;
        }}
        
        .section {{
            background: white;
            border-radius: 10px;
            padding: 30px;
            margin-bottom: 30px;
            box-shadow: 0 2px 8px rgba(0,0,0,0.1);
        }}
        
        .section-title {{
            font-size: 1.8em;
            color: #2c3e50;
            margin-bottom: 20px;
            padding-bottom: 10px;
            border-bottom: 2px solid #e9ecef;
        }}
        
        .card-grid {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
            gap: 20px;
            margin-bottom: 30px;
        }}
        
        .card {{
            background: #f8f9fa;
            border: 1px solid #e9ecef;
            border-radius: 8px;
            padding: 20px;
            transition: box-shadow 0.3s;
        }}
        
        .card:hover {{
            box-shadow: 0 4px 12px rgba(0,0,0,0.15);
        }}
        
        .metric-name {{
            font-size: 1.3em;
            font-weight: 600;
            color: #495057;
            margin-bottom: 10px;
        }}
        
        .metric-value {{
            font-size: 2.5em;
            font-weight: 700;
            color: #2c3e50;
            margin: 15px 0;
        }}
        
        .metric-details {{
            color: #6c757d;
            font-size: 0.95em;
            line-height: 1.5;
        }}
        
        .badge {{
            display: inline-block;
            padding: 4px 12px;
            border-radius: 4px;
            font-size: 0.9em;
            font-weight: 500;
            margin-bottom: 10px;
        }}
        
        .badge.success {{
            background: #d4edda;
            color: #155724;
        }}
        
        .badge.info {{
            background: #d1ecf1;
            color: #0c5460;
        }}
        
        .table {{
            width: 100%;
            border-collapse: collapse;
            margin: 20px 0;
            background: white;
            border-radius: 8px;
            overflow: hidden;
        }}
        
        .table thead {{
            background: #f8f9fa;
        }}
        
        .table th {{
            padding: 15px;
            text-align: left;
            font-weight: 600;
            color: #495057;
            border-bottom: 2px solid #dee2e6;
        }}
        
        .table td {{
            padding: 12px 15px;
            text-align: left;
            border-bottom: 1px solid #e9ecef;
        }}
        
        .table tbody tr:last-child td {{
            border-bottom: none;
        }}
        
        .table tbody tr:hover {{
            background: #f1f5f9;
        }}
        
        .bar-container {{
            margin: 10px 0;
            background: #f1f5f9;
            border-radius: 4px;
            padding: 3px;
        }}
        
        .bar {{
            height: 24px;
            border-radius: 4px;
            transition: width 0.3s;
        }}
        
        .info-grid {{
            display: grid;
            grid-template-columns: 1fr 1fr 1fr;
            gap: 15px;
            margin-top: 10px;
        }}
        
        .info-item {{
            padding: 10px;
            background: #f8f9fa;
            border-radius: 4px;
        }}
        
        .info-label {{
            font-size: 0.85em;
            color: #6c757d;
            font-weight: 500;
        }}
        
        .info-value {{
            font-size: 1.1em;
            font-weight: 600;
            color: #495057;
        }}
        
        .legend {{
            background: #fff3cd;
            border: 1px solid #d4edda;
            border-radius: 6px;
            padding: 15px;
            margin-top: 20px;
            font-size: 0.9em;
        }}
        
        .legend-item {{
            display: flex;
            align-items: center;
            margin-bottom: 8px;
        }}
        
        .legend-color {{
            width: 20px;
            height: 20px;
            border-radius: 4px;
            margin-right: 10px;
            flex-shrink: 0;
        }}
        
        .footer {{
            text-align: center;
            margin-top: 40px;
            padding-top: 20px;
            border-top: 1px solid #e9ecef;
            color: #6c757d;
            font-size: 0.9em;
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>📊 Cortex Mem 评估报告</h1>
            <p>生成时间: {datetime.now().strftime('%Y年%m月%d日 %H:%M')}</p>
            <div>
                <span class="badge success">数据集</span>
                <span class="badge info">150 个问题</span>
            </div>
        </div>
        
        <!-- 总体指标概览 -->
        <div class="section">
            <h2 class="section-title">📈 总体指标概览</h2>
            
            <div class="card-grid">
"""
    
    # 生成核心指标卡片
    key_metrics = [
        ('recall_at_1', '检索覆盖率 @1'),
        ('precision_at_1', '检索精确度 @1'),
        ('mrr', '排名质量'),
        ('answer_semantic_similarity', '答案语义相似度'),
        ('answer_exact_match', '精确匹配率')
    ]
    
    for metric_key, label in key_metrics:
        if metric_key in overall:
            metric_data = overall[metric_key]
            html_content += f"""
                <div class="card">
                    <div class="metric-name">{label}</div>
                    <div class="metric-value">{format_value(metric_data['mean'], 3)}</div>
                    <div class="metric-details">
                        <div>标准差: ±{format_value(metric_data['std'], 3)}</div>
                        <div>中位数: {format_value(metric_data['median'], 3)}</div>
                        <div>样本数: {metric_data['count']}</div>
                        <div style="margin-top: 10px; padding: 10px; background: {get_rating_color(metric_data['mean'])}; color: white; border-radius: 4px;">
                            评级: {get_rating_label(metric_data['mean'])}
                        </div>
                    </div>
                </div>
"""
    
    html_content += """
            </div>
        </div>
"""
    
    # 指标对比表格
    html_content += """
        <div class="section">
            <h2 class="section-title">📊 指标对比表格</h2>
            
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
            rating = get_rating_label(metric_data['mean'])
            
            ci_low, ci_high = metric_data['confidence_interval_95']
            
            html_content += f"""
                    <tr>
                        <td><strong>{info['name']}</strong></td>
                        <td>{info['category']}</td>
                        <td>{format_value(metric_data['mean'], 4)}</td>
                        <td>{format_value(metric_data['std'], 4)}</td>
                        <td>{format_value(ci_low, 4)} - {format_value(ci_high, 4)}</td>
                        <td>{metric_data['count']}</td>
                        <td style="color: {get_rating_color(metric_data['mean'])}; font-weight: 600;">{rating}</td>
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
            <h2 class="section-title">📂 分类指标详情</h2>
            
            <div class="card-grid">
"""
    
    category_names = {
        'category_1': '事实性问题',
        'category_2': '时间性问题',
        'category_3': '数量性问题'
    }
    
    for cat_key, cat_name in category_names.items():
        if cat_key in categories:
            cat_data = categories[cat_key]
            html_content += f"""
                <div class="card">
                    <h3 style="margin: 0 0 15px 0; color: #2c3e50;">{cat_name}</h3>
                    <div class="info-grid">
                        <div class="info-item">
                            <div class="info-label">问题数量</div>
                            <div class="info-value">{cat_data.get('recall_at_1', {}).get('count', 0)}</div>
                        </div>
                        <div class="info-item">
                            <div class="info-label">Recall@1</div>
                            <div class="info-value">{format_value(cat_data.get('recall_at_1', {}).get('mean', 0), 3)}</div>
                        </div>
                        <div class="info-item">
                            <div class="info-label">Precision@1</div>
                            <div class="info-value">{format_value(cat_data.get('precision_at_1', {}).get('mean', 0), 3)}</div>
                        </div>
                        <div class="info-item">
                            <div class="info-label">MRR</div>
                            <div class="info-value">{format_value(cat_data.get('mrr', {}).get('mean', 0), 3)}</div>
                        </div>
                        <div class="info-item">
                            <div class="info-label">语义相似度</div>
                            <div class="info-value">{format_value(cat_data.get('answer_semantic_similarity', {}).get('mean', 0), 3)}</div>
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
            <h2 class="section-title">📖 指标定义和说明</h2>
            
            <div class="card-grid">
"""
    
    for metric_key, info in metrics_info.items():
        if metric_key in overall:
            html_content += f"""
                <div class="card">
                    <div class="metric-name">{info['name']}</div>
                    <div class="metric-details">
                        <div style="margin-bottom: 8px;"><strong>类别:</strong> {info['category']}</div>
                        <div style="margin-bottom: 8px;"><strong>说明:</strong> {info['description']}</div>
                    </div>
                </div>
"""
    
    html_content += """
            </div>
        </div>
        
        <div class="legend">
            <h3 style="margin: 0 0 15px 0;">📊 评级说明</h3>
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
            <p>Cortex Mem 评估系统 v2.0</p>
            <p>生成时间: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}</p>
        </div>
    </div>
</body>
</html>
"""
    
    # 保存HTML文件
    with open(output_file, 'w', encoding='utf-8') as f:
        f.write(html_content)
    
    print(f"✅ HTML报告已生成: {output_file}")
    return output_file


def main():
    parser = argparse.ArgumentParser(description="生成评估报告HTML")
    parser.add_argument(
        "--results",
        type=str,
        default="results/cortex_mem_evaluated.json",
        help="评估结果文件路径"
    )
    parser.add_argument(
        "--output",
        type=str,
        default="report.html",
        help="输出的HTML文件路径"
    )
    
    args = parser.parse_args()
    
    # 生成HTML报告
    generate_html(args.results, args.output)
    
    print(f"\n📋 报告路径: {os.path.abspath(args.output)}")
    print("💡 在浏览器中打开: open " + os.path.abspath(args.output))


if __name__ == "__main__":
    main()
