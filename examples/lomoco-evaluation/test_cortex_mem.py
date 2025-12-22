#!/usr/bin/env python3
"""
测试脚本：验证 Cortex Mem 评估集成是否正常工作
"""

import os
import sys
import json
from pathlib import Path

# 添加 src 目录到 Python 路径
sys.path.insert(0, str(Path(__file__).parent / "src"))

from cortex_mem import (
    CortexMemAdd,
    CortexMemSearch,
    validate_config,
    check_openai_config
)


def test_config():
    """测试配置验证"""
    print("1. 测试配置验证...")
    
    config_path = "config.toml"
    if not os.path.exists(config_path):
        print(f"   ❌ 找不到配置文件: {config_path}")
        return False
    
    if validate_config(config_path):
        print(f"   ✅ 配置文件有效: {config_path}")
        return True
    else:
        print(f"   ❌ 配置文件无效: {config_path}")
        return False


def test_openai_config():
    """测试 OpenAI 配置"""
    print("2. 测试 OpenAI 配置...")
    
    config_path = "config.toml"
    if not os.path.exists(config_path):
        print(f"   ❌ 找不到配置文件: {config_path}")
        return False
    
    try:
        import toml
        config_data = toml.load(config_path)
        
        # 检查 llm 部分
        if "llm" not in config_data:
            print("   ❌ 配置文件中缺少 [llm] 部分")
            return False
        
        llm = config_data["llm"]
        if "api_key" not in llm:
            print("   ❌ [llm] 部分缺少 api_key")
            return False
        
        if llm["api_key"] == "your-openai-api-key-here":
            print("   ⚠️  请将 config.toml 中的 api_key 替换为实际的 OpenAI API 密钥")
            print("   ⚠️  测试将继续，但实际运行时需要有效的 API 密钥")
        
        # 检查 embedding 部分
        if "embedding" not in config_data:
            print("   ❌ 配置文件中缺少 [embedding] 部分")
            return False
        
        embedding = config_data["embedding"]
        if "api_key" not in embedding:
            print("   ❌ [embedding] 部分缺少 api_key")
            return False
        
        if embedding["api_key"] == "your-openai-api-key-here":
            print("   ⚠️  请将 config.toml 中的 embedding api_key 替换为实际的 OpenAI API 密钥")
            print("   ⚠️  测试将继续，但实际运行时需要有效的 API 密钥")
        
        print("   ✅ OpenAI 配置结构正确")
        return True
        
    except Exception as e:
        print(f"   ❌ 读取配置文件失败: {e}")
        return False


def test_cortex_mem_add():
    """测试 CortexMemAdd 类"""
    print("3. 测试 CortexMemAdd 类...")
    
    try:
        # 使用小样本数据测试
        test_data = [
            {
                "conversation": {
                    "speaker_a": "Alice",
                    "speaker_b": "Bob",
                    "session_1": [
                        {"speaker": "Alice", "text": "Hello, I'm Alice."},
                        {"speaker": "Bob", "text": "Hi Alice, I'm Bob."}
                    ],
                    "session_1_date_time": "2024-01-01 10:00:00"
                },
                "qa": []
            }
        ]
        
        # 创建测试数据文件
        test_data_path = "test_data.json"
        with open(test_data_path, "w") as f:
            json.dump(test_data, f)
        
        # 测试初始化
        add_manager = CortexMemAdd(data_path=test_data_path, batch_size=1)
        print("   ✅ CortexMemAdd 初始化成功")
        
        # 清理测试文件
        os.remove(test_data_path)
        
        # 测试析构函数
        del add_manager
        print("   ✅ 资源清理成功")
        
        return True
        
    except Exception as e:
        print(f"   ❌ CortexMemAdd 测试失败: {e}")
        return False


def test_cortex_mem_search():
    """测试 CortexMemSearch 类"""
    print("4. 测试 CortexMemSearch 类...")
    
    try:
        search_manager = CortexMemSearch(output_path="test_results.json", top_k=5)
        print("   ✅ CortexMemSearch 初始化成功")
        
        # 测试析构函数
        del search_manager
        
        # 清理测试文件
        if os.path.exists("test_results.json"):
            os.remove("test_results.json")
        
        print("   ✅ 资源清理成功")
        return True
        
    except Exception as e:
        print(f"   ❌ CortexMemSearch 测试失败: {e}")
        return False


def test_integration():
    """测试与 run_experiments.py 的集成"""
    print("5. 测试与 run_experiments.py 的集成...")
    
    try:
        from src.utils import TECHNIQUES, METHODS
        
        if "cortex_mem" in TECHNIQUES:
            print(f"   ✅ cortex_mem 已在 TECHNIQUES 列表中: {TECHNIQUES}")
        else:
            print(f"   ❌ cortex_mem 不在 TECHNIQUES 列表中: {TECHNIQUES}")
            return False
        
        if "add" in METHODS and "search" in METHODS:
            print(f"   ✅ METHODS 包含 add 和 search: {METHODS}")
        else:
            print(f"   ❌ METHODS 不完整: {METHODS}")
            return False
        
        return True
        
    except Exception as e:
        print(f"   ❌ 集成测试失败: {e}")
        return False


def main():
    """主测试函数"""
    print("=" * 60)
    print("Cortex Mem 评估集成测试")
    print("=" * 60)
    
    tests = [
        test_config,
        test_openai_config,
        test_cortex_mem_add,
        test_cortex_mem_search,
        test_integration
    ]
    
    passed = 0
    total = len(tests)
    
    for i, test_func in enumerate(tests, 1):
        print(f"\n测试 {i}/{total}:")
        if test_func():
            passed += 1
    
    print("\n" + "=" * 60)
    print(f"测试结果: {passed}/{total} 通过")
    
    if passed == total:
        print("✅ 所有测试通过！Cortex Mem 评估集成准备就绪。")
        print("\n下一步:")
        print("1. 确保 Qdrant 正在运行: docker run -p 6333:6333 qdrant/qdrant")
        print("2. 构建 cortex-mem-cli: cargo build --bin cortex-mem-cli")
        print("3. 运行评估: python run_experiments.py --technique_type cortex_mem --method add")
        return 0
    else:
        print("❌ 部分测试失败，请检查上述错误。")
        return 1


if __name__ == "__main__":
    sys.exit(main())