//! 报告生成器
//! 
//! 生成评估报告

use anyhow::Result;
use serde::Serialize;
use std::path::PathBuf;

/// 报告生成器
pub struct ReportGenerator {
    /// 输出目录
    output_dir: PathBuf,
}

impl ReportGenerator {
    /// 创建新的报告生成器
    pub fn new(output_dir: PathBuf) -> Self {
        Self { output_dir }
    }
    
    /// 生成JSON报告
    pub fn generate_json_report<T: Serialize>(&self, data: &T, filename: &str) -> Result<()> {
        let json = serde_json::to_string_pretty(data)?;
        let path = self.output_dir.join(filename);
        std::fs::write(path, json)?;
        Ok(())
    }
    
    /// 生成Markdown报告
    pub fn generate_markdown_report(&self, content: &str, filename: &str) -> Result<()> {
        let path = self.output_dir.join(filename);
        std::fs::write(path, content)?;
        Ok(())
    }
    
    /// 生成HTML报告
    pub fn generate_html_report(&self, content: &str, filename: &str) -> Result<()> {
        let html = format!(
            r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Cortex-Mem 评估报告</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 40px; line-height: 1.6; }}
        h1 {{ color: #333; border-bottom: 2px solid #4CAF50; }}
        h2 {{ color: #555; margin-top: 30px; }}
        .metric {{ background: #f5f5f5; padding: 15px; margin: 10px 0; border-radius: 5px; }}
        .score {{ font-size: 24px; font-weight: bold; color: #4CAF50; }}
        .warning {{ color: #ff9800; }}
        .error {{ color: #f44336; }}
        table {{ border-collapse: collapse; width: 100%; }}
        th, td {{ border: 1px solid #ddd; padding: 8px; text-align: left; }}
        th {{ background-color: #4CAF50; color: white; }}
        tr:nth-child(even) {{ background-color: #f2f2f2; }}
    </style>
</head>
<body>
    <h1>📊 Cortex-Mem 评估报告</h1>
    <p><strong>生成时间:</strong> {}</p>
    <hr>
    {}
</body>
</html>"#,
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"),
            content
        );
        
        let path = self.output_dir.join(filename);
        std::fs::write(path, html)?;
        Ok(())
    }
    
    /// 生成综合报告
    pub fn generate_comprehensive_report(
        &self,
        recall_metrics: Option<&serde_json::Value>,
        effectiveness_metrics: Option<&serde_json::Value>,
        performance_metrics: Option<&serde_json::Value>,
    ) -> Result<()> {
        let mut report = String::new();
        
        report.push_str("# Cortex-Mem 核心能力综合评估报告\n\n");
        report.push_str(&format!("**报告生成时间**: {}\n\n", 
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S")));
        
        // 执行摘要
        report.push_str("## 📋 执行摘要\n\n");
        report.push_str("本报告总结了 Cortex-Mem 核心能力的评估结果，包括召回率、记忆有效性和性能三个方面。\n\n");
        
        // 召回率评估结果
        if let Some(metrics) = recall_metrics {
            report.push_str("## 🔍 召回率评估结果\n\n");
            report.push_str("### 关键指标\n");
            report.push_str("| 指标 | 值 | 说明 |\n");
            report.push_str("|------|-----|------|\n");
            
            if let Some(precision) = metrics.get("precision_at_k") {
                if let Some(p1) = precision.get("1") {
                    report.push_str(&format!("| Precision@1 | {:.3} | 第一个结果的精确率 |\n", p1));
                }
                if let Some(p5) = precision.get("5") {
                    report.push_str(&format!("| Precision@5 | {:.3} | 前5个结果的精确率 |\n", p5));
                }
            }
            
            if let Some(recall) = metrics.get("recall_at_k") {
                if let Some(r5) = recall.get("5") {
                    report.push_str(&format!("| Recall@5 | {:.3} | 前5个结果的召回率 |\n", r5));
                }
            }
            
            if let Some(map) = metrics.get("mean_average_precision") {
                report.push_str(&format!("| MAP | {:.3} | 平均精确率均值 |\n", map));
            }
            
            if let Some(ndcg) = metrics.get("normalized_discounted_cumulative_gain") {
                report.push_str(&format!("| NDCG | {:.3} | 归一化折损累计增益 |\n", ndcg));
            }
            report.push_str("\n");
        }
        
        // 有效性评估结果
        if let Some(metrics) = effectiveness_metrics {
            report.push_str("## ✅ 记忆有效性评估结果\n\n");
            
            if let Some(overall) = metrics.get("overall_score") {
                report.push_str(&format!("### 综合得分: {:.2}/1.00\n\n", overall));
            }
            
            report.push_str("### 各维度得分\n");
            report.push_str("| 维度 | 得分 | 状态 |\n");
            report.push_str("|------|------|------|\n");
            
            if let Some(fact) = metrics.get("fact_extraction_accuracy") {
                if let Some(f1) = fact.get("f1_score") {
                    let score = f1.as_f64().unwrap_or(0.0);
                    let status = if score >= 0.9 { "✅ 优秀" } else if score >= 0.7 { "⚠️ 良好" } else { "❌ 需改进" };
                    report.push_str(&format!("| 事实提取 | {:.3} | {} |\n", score, status));
                }
            }
            
            if let Some(class) = metrics.get("classification_accuracy") {
                if let Some(accuracy) = class.get("accuracy") {
                    let score = accuracy.as_f64().unwrap_or(0.0);
                    let status = if score >= 0.9 { "✅ 优秀" } else if score >= 0.7 { "⚠️ 良好" } else { "❌ 需改进" };
                    report.push_str(&format!("| 记忆分类 | {:.3} | {} |\n", score, status));
                }
            }
            report.push_str("\n");
        }
        
        // 性能评估结果
        if let Some(_metrics) = performance_metrics {
            report.push_str("## ⚡ 性能评估结果\n\n");
            report.push_str("性能评估需要实际的 MemoryManager 实例才能运行。\n\n");
            report.push_str("### 支持的测试类型\n");
            report.push_str("1. **基准测试**: 测量基本操作性能\n");
            report.push_str("2. **负载测试**: 模拟不同并发用户\n");
            report.push_str("3. **压力测试**: 测试系统极限\n");
            report.push_str("4. **可扩展性测试**: 验证不同规模下的性能\n\n");
        }
        
        // 结论和建议
        report.push_str("## 🎯 结论与建议\n\n");
        
        if recall_metrics.is_some() || effectiveness_metrics.is_some() {
            report.push_str("### 优势\n");
            report.push_str("- 评估框架结构完整，覆盖核心能力维度\n");
            report.push_str("- 支持多种评估指标和测试场景\n");
            report.push_str("- 配置灵活，可根据需要调整评估参数\n\n");
            
            report.push_str("### 改进建议\n");
            report.push_str("1. **集成实际系统**: 将 MemoryManager 实例注入评估框架\n");
            report.push_str("2. **扩展测试数据集**: 增加更多样化的测试用例\n");
            report.push_str("3. **优化评估算法**: 改进指标计算方法的准确性\n");
            report.push_str("4. **添加自动化**: 实现持续集成和自动化评估\n\n");
        } else {
            report.push_str("### 框架状态\n");
            report.push_str("✅ **框架就绪**: 评估框架已实现，结构完整\n");
            report.push_str("⚠️ **需要集成**: 需要提供 MemoryManager 实例以运行实际评估\n");
            report.push_str("📊 **支持全面**: 覆盖召回率、有效性、性能三个维度的评估\n\n");
        }
        
        report.push_str("### 下一步计划\n");
        report.push_str("1. 运行实际评估获取基准数据\n");
            report.push_str("2. 根据评估结果优化系统实现\n");
            report.push_str("3. 建立定期评估机制\n");
            report.push_str("4. 扩展评估场景和测试用例\n\n");
        
        report.push_str("---\n");
        report.push_str("*报告由 Cortex-Mem 评估框架生成*\n");
        
        // 生成各种格式的报告
        self.generate_markdown_report(&report, "comprehensive_report.md")?;
        self.generate_html_report(&report, "comprehensive_report.html")?;
        
        // 生成JSON格式的原始数据
        let json_data = serde_json::json!({
            "report_generated_at": chrono::Utc::now().to_rfc3339(),
            "recall_metrics": recall_metrics,
            "effectiveness_metrics": effectiveness_metrics,
            "performance_metrics": performance_metrics,
            "report_version": "1.0.0"
        });
        
        self.generate_json_report(&json_data, "comprehensive_report.json")?;
        
        Ok(())
    }
}
