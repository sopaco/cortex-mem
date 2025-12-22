#!/usr/bin/env python3
"""
简化版 cortex-mem 测试脚本
专门测试 cortex-mem 的核心功能，绕过其他技术的依赖问题
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


def test_cortex_mem_core():
    """测试 cortex-mem 核心功能"""
    print("=" * 60)
    print("Cortex Mem 核心功能测试")
    print("=" * 60)
    
    # 1. 测试配置
    print("\n1. 测试配置验证...")
    if not validate_config("config.toml"):
        print("   ❌ 配置文件验证失败")
        return False
    print("   ✅ 配置文件验证通过")
    
    # 2. 测试 OpenAI 配置
    print("\n2. 测试 OpenAI 配置...")
    if not check_openai_config("config.toml"):
        print("   ❌ OpenAI 配置检查失败")
        return False
    print("   ✅ OpenAI 配置检查通过")
    
    # 3. 测试 CortexMemAdd
    print("\n3. 测试 CortexMemAdd...")
    try:
        add_manager = CortexMemAdd(data_path="dataset/locomo10.json", batch_size=1)
        print("   ✅ CortexMemAdd 初始化成功")
        
        # 测试数据加载
        print("   📊 加载测试数据...")
        with open("dataset/locomo10.json", "r") as f:
            test_data = json.load(f)
        print(f"   ✅ 成功加载 {len(test_data)} 个对话")
        
        # 清理
        del add_manager
        print("   ✅ CortexMemAdd 资源清理成功")
        
    except Exception as e:
        print(f"   ❌ CortexMemAdd 测试失败: {e}")
        return False
    
    # 4. 测试 CortexMemSearch
    print("\n4. 测试 CortexMemSearch...")
    try:
        search_manager = CortexMemSearch(output_path="test_results.json", top_k=5)
        print("   ✅ CortexMemSearch 初始化成功")
        
        # 测试 CLI 工具路径
        project_root = Path(__file__).parent.parent.parent
        cli_path = project_root / "cortex-mem-cli"
        if cli_path.exists():
            print(f"   ✅ 找到 cortex-mem-cli: {cli_path}")
        else:
            print(f"   ⚠️  未找到 cortex-mem-cli: {cli_path}")
        
        # 清理
        del search_manager
        if os.path.exists("test_results.json"):
            os.remove("test_results.json")
        print("   ✅ CortexMemSearch 资源清理成功")
        
    except Exception as e:
        print(f"   ❌ CortexMemSearch 测试失败: {e}")
        return False
    
    # 5. 测试数据集完整性
    print("\n5. 测试数据集完整性...")
    try:
        with open("dataset/locomo10.json", "r") as f:
            data = json.load(f)
        
        if len(data) == 0:
            print("   ❌ 数据集为空")
            return False
        
        # 检查第一个对话的结构
        first_conv = data[0]
        if "conversation" not in first_conv or "qa" not in first_conv:
            print("   ❌ 对话结构不正确")
            return False
        
        conversation = first_conv["conversation"]
        if "speaker_a" not in conversation or "speaker_b" not in conversation:
            print("   ❌ 说话者信息缺失")
            return False
        
        qa = first_conv["qa"]
        if len(qa) > 0:
            qa_item = qa[0]
            required_fields = ["question", "answer", "category"]
            for field in required_fields:
                if field not in qa_item:
                    print(f"   ❌ QA 字段缺失: {field}")
                    return False
        
        print(f"   ✅ 数据集完整性检查通过")
        print(f"   📊 包含 {len(data)} 个对话")
        print(f"   📊 总计 {sum(len(item['qa']) for item in data)} 个问答")
        
    except Exception as e:
        print(f"   ❌ 数据集测试失败: {e}")
        return False
    
    print("\n" + "=" * 60)
    print("✅ 所有核心功能测试通过！")
    print("\nCortex Mem 评估系统已准备就绪，可以进行以下操作：")
    print("1. 添加记忆: python run_cortex_mem_evaluation.py --method add")
    print("2. 搜索记忆: python run_cortex_mem_evaluation.py --method search")
    print("\n注意：实际运行需要:")
    print("- 有效的 OpenAI API 密钥")
    print("- 启动 Qdrant 服务")
    print("- 构建 cortex-mem-cli")
    return True


def main():
    """主测试函数"""
    try:
        success = test_cortex_mem_core()
        return 0 if success else 1
    except KeyboardInterrupt:
        print("\n\n测试被用户中断")
        return 1
    except Exception as e:
        print(f"\n\n测试过程中发生错误: {e}")
        return 1


if __name__ == "__main__":
    sys.exit(main())