#!/usr/bin/env python3
"""
专门运行 cortex-mem 评估的脚本
避免导入其他技术的依赖包，只专注于 cortex-mem
"""

import argparse
import json
import os
import sys
from pathlib import Path

# 添加 src 目录到 Python 路径
sys.path.insert(0, str(Path(__file__).parent / "src"))

from cortex_mem import CortexMemAdd, CortexMemSearch


def run_add_experiment():
    """运行添加记忆的实验"""
    print("=" * 60)
    print("Cortex Mem 添加记忆实验")
    print("=" * 60)
    
    try:
        # 初始化 CortexMemAdd
        print("🔄 初始化 CortexMemAdd...")
        add_manager = CortexMemAdd(data_path="dataset/locomo10.json", batch_size=1)
        print("✅ CortexMemAdd 初始化成功")
        
        # 处理所有对话
        print("🔄 开始添加记忆到 Cortex Mem...")
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


def run_search_experiment():
    """运行搜索记忆的实验"""
    print("=" * 60)
    print("Cortex Mem 搜索记忆实验")
    print("=" * 60)
    
    try:
        # 初始化 CortexMemSearch
        print("🔄 初始化 CortexMemSearch...")
        search_manager = CortexMemSearch(
            output_path="results/cortex_mem_results.json", 
            top_k=10
        )
        print("✅ CortexMemSearch 初始化成功")
        
        # 处理数据文件并生成结果
        print("🔄 开始搜索记忆并回答问题...")
        search_manager.process_data_file("dataset/locomo10.json")
        print("✅ 搜索记忆实验完成")
        
        # 检查结果文件
        if os.path.exists("results/cortex_mem_results.json"):
            with open("results/cortex_mem_results.json", "r") as f:
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
    parser = argparse.ArgumentParser(description="运行 Cortex Mem 评估")
    parser.add_argument(
        "--method", 
        choices=["add", "search"], 
        required=True,
        help="要运行的方法: add (添加记忆) 或 search (搜索记忆)"
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
    
    print("🚀 开始运行 Cortex Mem 评估")
    print(f"📋 方法: {args.method}")
    
    success = False
    
    if args.method == "add":
        success = run_add_experiment()
    elif args.method == "search":
        success = run_search_experiment()
    
    if success:
        print("\n🎉 评估成功完成！")
        print("\n📋 后续步骤:")
        print("1. 运行评估: python evals.py --input_file results/cortex_mem_results.json --output_file results/cortex_mem_evaluated.json")
        print("2. 生成分数: python generate_scores.py")
    else:
        print("\n❌ 评估失败，请检查错误信息")
        sys.exit(1)


if __name__ == "__main__":
    main()