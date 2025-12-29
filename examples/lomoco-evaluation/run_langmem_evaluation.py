#!/usr/bin/env python3
"""
专门运行 LangMem 评估的脚本
使用与 cortex-mem 完全相同的测试方法、测试数据、大模型配置
"""

import argparse
import json
import os
import sys
from pathlib import Path

# 添加 src 目录到 Python 路径
sys.path.insert(0, str(Path(__file__).parent / "src"))

from langmem_eval import LangMemAdd, LangMemSearch


def run_add_experiment(data_path="dataset/locomo50.json"):
    """运行添加记忆的实验"""
    print("=" * 60)
    print("LangMem 添加记忆实验")
    print("=" * 60)
    
    try:
        # 初始化 LangMemAdd
        print("🔄 初始化 LangMemAdd...")
        add_manager = LangMemAdd(data_path=data_path, batch_size=1)
        print("✅ LangMemAdd 初始化成功")
        
        # 处理所有对话
        print("🔄 开始添加记忆到 LangMem...")
        add_manager.process_all_conversations()
        print("✅ 所有记忆添加完成")
        
        # 清理资源
        del add_manager
        print("🧹 资源清理完成")
        
        print("\n✅ 添加记忆实验完成！")
        return True
        
    except Exception as e:
        print(f"❌ 添加记忆实验失败: {e}")
        return False


def run_search_experiment(data_path="dataset/locomo50.json", top_k=10):
    """运行搜索记忆的实验"""
    print("=" * 60)
    print("LangMem 搜索记忆实验")
    print("=" * 60)
    
    try:
        # 初始化 LangMemSearch
        print("🔄 初始化 LangMemSearch...")
        search_manager = LangMemSearch(
            output_path="results/langmem_results.json", 
            top_k=top_k
        )
        print("✅ LangMemSearch 初始化成功")
        
        # 处理数据文件并生成结果
        print("🔄 开始搜索记忆并回答问题...")
        search_manager.process_data_file(data_path)
        print("✅ 搜索记忆实验完成")
        
        # 检查结果文件
        if os.path.exists("results/langmem_results.json"):
            with open("results/langmem_results.json", "r") as f:
                results = json.load(f)
            print(f"📊 生成了 {len(results)} 个结果")
        
        # 清理资源
        del search_manager
        print("🧹 资源清理完成")
        
        print("\n✅ 搜索记忆实验完成！")
        return True
        
    except Exception as e:
        print(f"❌ 搜索记忆实验失败: {e}")
        return False


def main():
    """主函数"""
    parser = argparse.ArgumentParser(description="运行 LangMem 评估")
    parser.add_argument(
        "--method", 
        choices=["add", "search"], 
        required=True,
        help="要运行的方法: add (添加记忆) 或 search (搜索记忆)"
    )
    parser.add_argument(
        "--data",
        type=str,
        default="dataset/locomo50.json",
        help="数据集文件路径 (默认: dataset/locomo50.json)"
    )
    parser.add_argument(
        "--top_k", 
        type=int, 
        default=10,
        help="搜索时返回的记忆数量"
    )
    
    args = parser.parse_args()
    
    # 创建 results 目录
    os.makedirs("results", exist_ok=True)
    
    print("🚀 开始运行 LangMem 评估")
    print(f"📋 方法: {args.method}")
    print(f"📊 数据集: {args.data}")
    
    success = False
    
    if args.method == "add":
        success = run_add_experiment(args.data)
    elif args.method == "search":
        success = run_search_experiment(args.data, args.top_k)
    
    if success:
        print("\n🎉 评估成功完成！")
        print("\n📋 后续步骤:")
        print("1. 运行评估: python -m metrics.memory_evaluation \\")
        print(f"   --results results/langmem_results.json \\")
        print(f"   --dataset {args.data} \\")
        print(f"   --output results/langmem_evaluated.json")
        print("\n2. 生成HTML报告:")
        print("   python generate_report.py \\")
        print(f"   --results results/langmem_evaluated.json \\")
        print(f"   --output results/langmem_report.html")
    else:
        print("\n❌ 评估失败，请检查错误信息")
        sys.exit(1)


if __name__ == "__main__":
    main()