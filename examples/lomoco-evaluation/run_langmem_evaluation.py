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
    print("LangMem Add Memory Experiment")
    print("=" * 60)

    try:
        # 初始化 LangMemAdd
        print("Initializing LangMemAdd...")
        add_manager = LangMemAdd(data_path=data_path, batch_size=1)
        print("LangMemAdd initialized successfully")

        # 处理所有对话
        print("Adding memories to LangMem...")
        add_manager.process_all_conversations()
        print("All memories added successfully")

        # 清理资源
        del add_manager
        print("Resources cleaned up")

        print("\nAdd memory experiment completed!")
        return True

    except Exception as e:
        print(f"Add memory experiment failed: {e}")
        return False


def run_search_experiment(data_path="dataset/locomo50.json", top_k=10):
    """运行搜索记忆的实验"""
    print("=" * 60)
    print("LangMem Search Memory Experiment")
    print("=" * 60)

    try:
        # 初始化 LangMemSearch
        print("Initializing LangMemSearch...")
        search_manager = LangMemSearch(
            output_path="results/langmem_results.json",
            top_k=top_k
        )
        print("LangMemSearch initialized successfully")

        # 处理数据文件并生成结果
        print("Searching memories and answering questions...")
        search_manager.process_data_file(data_path)
        print("Search memory experiment completed")

        # 检查结果文件
        if os.path.exists("results/langmem_results.json"):
            with open("results/langmem_results.json", "r") as f:
                results = json.load(f)
            print(f"Generated {len(results)} results")

        # 清理资源
        del search_manager
        print("Resources cleaned up")

        print("\nSearch memory experiment completed!")
        return True

    except Exception as e:
        print(f"Search memory experiment failed: {e}")
        return False


def main():
    """主函数"""
    parser = argparse.ArgumentParser(description="Run LangMem evaluation")
    parser.add_argument(
        "--method",
        choices=["add", "search"],
        required=True,
        help="Method to run: add (add memories) or search (search memories)"
    )
    parser.add_argument(
        "--data",
        type=str,
        default="dataset/locomo50.json",
        help="Dataset file path (default: dataset/locomo50.json)"
    )
    parser.add_argument(
        "--top_k",
        type=int,
        default=10,
        help="Number of memories to return during search"
    )

    args = parser.parse_args()

    # 创建 results 目录
    os.makedirs("results", exist_ok=True)

    print("Starting LangMem evaluation")
    print(f"Method: {args.method}")
    print(f"Dataset: {args.data}")

    success = False

    if args.method == "add":
        success = run_add_experiment(args.data)
    elif args.method == "search":
        success = run_search_experiment(args.data, args.top_k)

    if success:
        print("\nEvaluation completed successfully!")
        print("\nNext steps:")
        print("1. Run evaluation: python -m metrics.memory_evaluation \\")
        print(f"   --results results/langmem_results.json \\")
        print(f"   --dataset {args.data} \\")
        print(f"   --output results/langmem_evaluated.json")
        print("\n2. Generate HTML report:")
        print("   python generate_report.py \\")
        print(f"   --results results/langmem_evaluated.json \\")
        print(f"   --output results/langmem_report.html")
    else:
        print("\nEvaluation failed, please check error messages")
        sys.exit(1)


if __name__ == "__main__":
    main()