#!/usr/bin/env python3
"""
修复CLI参数问题
移除add.py中不支持的--topics和--keywords参数
"""

import re
import sys
from pathlib import Path


def fix_add_py():
    """修复add.py中的CLI参数"""
    add_py_path = Path("src/cortex_mem/search.py")
    
    if not add_py_path.exists():
        print(f"❌ 文件不存在: {add_py_path}")
        return False
    
    try:
        with open(add_py_path, "r", encoding="utf-8") as f:
            content = f.read()
        
        # 移除--topics和--keywords参数
        # 匹配: args.extend(["--topics", ",".join(topics)])
        pattern1 = r'\s*if topics:\s*\n\s*args\.extend\(\["--topics", ","\.join\(topics\)\]\)\s*\n'
        content = re.sub(pattern1, '', content, flags=re.MULTILINE)
        
        # 匹配: args.extend(["--keywords", ",".join(keywords)])
        pattern2 = r'\s*if keywords:\s*\n\s*args\.extend\(\["--keywords", ","\.join\(keywords\)\]\)\s*\n'
        content = re.sub(pattern2, '', content, flags=re.MULTILINE)
        
        # 保存修改后的文件
        with open(add_py_path, "w", encoding="utf-8") as f:
            f.write(content)
        
        print("✅ 成功修复add.py中的CLI参数")
        return True
        
    except Exception as e:
        print(f"❌ 修复失败: {e}")
        return False


def fix_cli_paths():
    """修复CLI路径问题"""
    files_to_fix = [
        "src/cortex_mem/search.py",
        "src/cortex_mem/search.py"
    ]
    
    for file_path in files_to_fix:
        path = Path(file_path)
        if not path.exists():
            continue
            
        try:
            with open(path, "r", encoding="utf-8") as f:
                content = f.read()
            
            # 修复CLI路径
            old_pattern = r'project_root = Path\(__file__\)\.parent\.parent\.parent\.parent\.parent'
            new_pattern = r'project_root = Path(__file__).parent.parent.parent.parent'
            content = re.sub(old_pattern, new_pattern, content)
            
            with open(path, "w", encoding="utf-8") as f:
                f.write(content)
            
            print(f"✅ 修复了 {file_path} 中的CLI路径")
            
        except Exception as e:
            print(f"❌ 修复 {file_path} 失败: {e}")


def main():
    """主函数"""
    print("🔧 修复CLI参数问题...")
    
    # 修复CLI路径
    fix_cli_paths()
    
    # 修复add.py中的参数
    fix_add_py()
    
    print("✅ 修复完成！")


if __name__ == "__main__":
    main()