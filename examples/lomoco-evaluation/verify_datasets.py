#!/usr/bin/env python3
"""
验证生成的数据集文件
"""

import json
import sys
from pathlib import Path


def verify_dataset(filename, expected_format):
    """验证数据集文件格式"""
    print(f"\n验证文件: {filename}")
    
    if not Path(filename).exists():
        print(f"❌ 文件不存在: {filename}")
        return False
    
    try:
        with open(filename, 'r', encoding='utf-8') as f:
            data = json.load(f)
        
        print(f"✅ JSON 格式正确")
        print(f"📊 数据量: {len(data)} 条")
        
        # 验证基本结构
        if expected_format == "original":
            # 原始格式: [{conversation, qa}]
            if len(data) > 0:
                if "conversation" not in data[0] or "qa" not in data[0]:
                    print("❌ 原始格式错误：缺少 conversation 或 qa 字段")
                    return False
                
                conversation = data[0]["conversation"]
                if "speaker_a" not in conversation or "speaker_b" not in conversation:
                    print("❌ 对话格式错误：缺少 speaker_a 或 speaker_b")
                    return False
                
                qa = data[0]["qa"]
                if len(qa) > 0:
                    qa_item = qa[0]
                    required_qa_fields = ["question", "answer", "category"]
                    for field in required_qa_fields:
                        if field not in qa_item:
                            print(f"❌ QA 格式错误：缺少 {field} 字段")
                            return False
                
                print(f"✅ 原始格式正确")
                print(f"👥 对话参与者: {conversation['speaker_a']} & {conversation['speaker_b']}")
                print(f"❓ 问答数量: {len(qa)}")
            else:
                print("⚠️ 数据为空")
        
        elif expected_format == "rag":
            # RAG 格式: [{conversation, question, answer, category}]
            if len(data) > 0:
                required_fields = ["conversation", "question", "answer", "category"]
                for field in required_fields:
                    if field not in data[0]:
                        print(f"❌ RAG 格式错误：缺少 {field} 字段")
                        return False
                
                conversation = data[0]["conversation"]
                if "speaker_a" not in conversation or "speaker_b" not in conversation:
                    print("❌ 对话格式错误：缺少 speaker_a 或 speaker_b")
                    return False
                
                print(f"✅ RAG 格式正确")
                print(f"👥 对话参与者: {conversation['speaker_a']} & {conversation['speaker_b']}")
                print(f"❓ 示例问题: {data[0]['question'][:50]}...")
            
        # 统计信息
        categories = set()
        speakers = set()
        total_qa = 0
        
        for item in data:
            if expected_format == "original":
                qa_items = item.get("qa", [])
                total_qa += len(qa_items)
                for qa_item in qa_items:
                    categories.add(str(qa_item.get("category", "")))
            
            conv = item.get("conversation", {})
            speakers.add(conv.get("speaker_a", ""))
            speakers.add(conv.get("speaker_b", ""))
            
            if expected_format == "rag":
                categories.add(str(item.get("category", "")))
                total_qa += 1
        
        print(f"📈 统计信息:")
        print(f"   参与者: {len(speakers)} 人 ({', '.join(sorted(speakers))})")
        print(f"   类别: {sorted(categories)}")
        print(f"   总问答: {total_qa} 个")
        
        return True
        
    except json.JSONDecodeError as e:
        print(f"❌ JSON 解析错误: {e}")
        return False
    except Exception as e:
        print(f"❌ 验证错误: {e}")
        return False


def main():
    """主验证函数"""
    print("=" * 60)
    print("LOCOMO 数据集验证")
    print("=" * 60)
    
    datasets = [
        ("dataset/locomo10.json", "original"),
        ("dataset/locomo10_rag.json", "rag")
    ]
    
    all_passed = True
    
    for filename, format_type in datasets:
        if not verify_dataset(filename, format_type):
            all_passed = False
    
    print("\n" + "=" * 60)
    if all_passed:
        print("✅ 所有数据集验证通过！")
        print("\n数据集特点:")
        print("• 包含 10 个不同的对话场景")
        print("• 涵盖工作、学习、生活、兴趣等多个领域")
        print("• 包含 4 个不同类别的问答")
        print("• 支持原始格式和 RAG 格式")
        print("• 适合测试内存recall和理解能力")
        print("\n可以使用以下命令运行评估:")
        print("python run_experiments.py --technique_type cortex_mem --method add")
        print("python run_experiments.py --technique_type cortex_mem --method search")
        return 0
    else:
        print("❌ 部分数据集验证失败！")
        return 1


if __name__ == "__main__":
    sys.exit(main())