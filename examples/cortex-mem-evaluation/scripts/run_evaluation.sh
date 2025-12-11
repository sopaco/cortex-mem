#!/bin/bash

# Cortex-Mem 评估脚本
# 运行完整的评估流程

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 日志函数
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# 显示帮助
show_help() {
    echo "Cortex-Mem 评估脚本"
    echo ""
    echo "用法: $0 [选项]"
    echo ""
    echo "选项:"
    echo "  -h, --help            显示此帮助信息"
    echo "  -m, --mode MODE       评估模式 (all, recall, effectiveness, performance)"
    echo "  -c, --config FILE     配置文件路径 (默认: config/evaluation_config.toml)"
    echo "  -o, --output DIR      输出目录 (默认: results)"
    echo "  -g, --generate        生成测试数据集"
    echo "  -s, --size SIZE       数据集大小 (默认: 100)"
    echo "  -v, --verbose         详细输出"
    echo ""
    echo "示例:"
    echo "  $0 --mode all                        运行完整评估"
    echo "  $0 --mode recall --output my_results 仅运行召回率评估"
    echo "  $0 --generate --size 200             生成测试数据集"
}

# 默认参数
MODE="all"
CONFIG_FILE="examples/cortex-mem-evaluation/config/evaluation_config.toml"
OUTPUT_DIR="results"
GENERATE_DATASET=false
DATASET_SIZE=100
VERBOSE=false

# 解析命令行参数
while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            show_help
            exit 0
            ;;
        -m|--mode)
            MODE="$2"
            shift 2
            ;;
        -c|--config)
            CONFIG_FILE="$2"
            shift 2
            ;;
        -o|--output)
            OUTPUT_DIR="$2"
            shift 2
            ;;
        -g|--generate)
            GENERATE_DATASET=true
            shift
            ;;
        -s|--size)
            DATASET_SIZE="$2"
            shift 2
            ;;
        -v|--verbose)
            VERBOSE=true
            shift
            ;;
        *)
            log_error "未知参数: $1"
            show_help
            exit 1
            ;;
    esac
done

# 检查必要工具
check_dependencies() {
    log_info "检查依赖..."
    
    # 检查 Rust
    if ! command -v cargo &> /dev/null; then
        log_error "未找到 cargo，请安装 Rust"
        exit 1
    fi
    
    # 检查是否在项目目录中
    if [ ! -f "Cargo.toml" ]; then
        log_error "未找到 Cargo.toml，请在项目目录中运行此脚本"
        exit 1
    fi
    
    log_success "依赖检查通过"
}

# 生成测试数据集
generate_dataset() {
    log_info "生成测试数据集 (大小: $DATASET_SIZE)..."
    
    # 创建数据目录
    mkdir -p data/test_cases
    mkdir -p data/ground_truth
    
    # 运行数据集生成
    if [ "$VERBOSE" = true ]; then
        cargo run -p cortex-mem-evaluation -- generate-dataset --dataset-type all --output-dir data --size "$DATASET_SIZE"
    else
        cargo run -p cortex-mem-evaluation -- generate-dataset --dataset-type all --output-dir data --size "$DATASET_SIZE" > /dev/null 2>&1
    fi
    
    if [ $? -eq 0 ]; then
        log_success "测试数据集生成完成"
        
        # 验证数据集
        log_info "验证数据集..."
        cargo run -p cortex-mem-evaluation -- validate-dataset --dataset-path data/test_cases/recall_test_cases.json --dataset-type recall
        cargo run -p cortex-mem-evaluation -- validate-dataset --dataset-path data/test_cases/effectiveness_test_cases.json --dataset-type effectiveness
    else
        log_error "数据集生成失败"
        exit 1
    fi
}

# 运行评估
run_evaluation() {
    log_info "开始 $MODE 评估..."
    
    # 创建输出目录
    mkdir -p "$OUTPUT_DIR"
    mkdir -p "$OUTPUT_DIR/reports"
    mkdir -p "$OUTPUT_DIR/visualizations"
    
    # 根据模式运行评估
    case $MODE in
        all)
            log_info "运行完整评估..."
            if [ "$VERBOSE" = true ]; then
                cargo run -p cortex-mem-evaluation -- run --config "$CONFIG_FILE" --output-dir "$OUTPUT_DIR"
            else
                cargo run -p cortex-mem-evaluation -- run --config "$CONFIG_FILE" --output-dir "$OUTPUT_DIR" 2>&1 | tee "$OUTPUT_DIR/evaluation.log"
            fi
            ;;
        recall)
            log_info "运行召回率评估..."
            if [ "$VERBOSE" = true ]; then
                cargo run -p cortex-mem-evaluation -- recall --config "$CONFIG_FILE" --output-dir "$OUTPUT_DIR"
            else
                cargo run -p cortex-mem-evaluation -- recall --config "$CONFIG_FILE" --output-dir "$OUTPUT_DIR" 2>&1 | tee "$OUTPUT_DIR/recall_evaluation.log"
            fi
            ;;
        effectiveness)
            log_info "运行有效性评估..."
            if [ "$VERBOSE" = true ]; then
                cargo run -p cortex-mem-evaluation -- effectiveness --config "$CONFIG_FILE" --output-dir "$OUTPUT_DIR"
            else
                cargo run -p cortex-mem-evaluation -- effectiveness --config "$CONFIG_FILE" --output-dir "$OUTPUT_DIR" 2>&1 | tee "$OUTPUT_DIR/effectiveness_evaluation.log"
            fi
            ;;
        performance)
            log_info "运行性能评估..."
            if [ "$VERBOSE" = true ]; then
                cargo run -p cortex-mem-evaluation -- performance --config "$CONFIG_FILE" --output-dir "$OUTPUT_DIR"
            else
                cargo run -p cortex-mem-evaluation -- performance --config "$CONFIG_FILE" --output-dir "$OUTPUT_DIR" 2>&1 | tee "$OUTPUT_DIR/performance_evaluation.log"
            fi
            ;;
        *)
            log_error "未知的评估模式: $MODE"
            show_help
            exit 1
            ;;
    esac
    
    if [ $? -eq 0 ]; then
        log_success "$MODE 评估完成"
        
        # 显示结果文件
        log_info "评估结果:"
        find "$OUTPUT_DIR" -name "*.json" -o -name "*.md" -o -name "*.html" | while read -r file; do
            echo "  - $file"
        done
    else
        log_error "评估失败"
        exit 1
    fi
}

# 生成报告
generate_report() {
    log_info "生成评估报告..."
    
    # 检查是否有结果文件
    if [ ! -f "$OUTPUT_DIR/comprehensive_report.json" ]; then
        log_warning "未找到综合报告，跳过报告生成"
        return
    fi
    
    # 这里可以添加报告生成逻辑
    # 例如：转换JSON为Markdown、生成图表等
    
    log_success "报告生成完成"
}

# 主函数
main() {
    log_info "========================================"
    log_info "    Cortex-Mem 核心能力评估框架"
    log_info "========================================"
    
    # 检查依赖
    check_dependencies
    
    # 构建项目
    log_info "构建项目..."
    if [ "$VERBOSE" = true ]; then
        cargo build --release
    else
        cargo build --release > /dev/null 2>&1
    fi
    
    if [ $? -ne 0 ]; then
        log_error "构建失败"
        exit 1
    fi
    log_success "构建完成"
    
    # 生成数据集（如果需要）
    if [ "$GENERATE_DATASET" = true ]; then
        generate_dataset
    fi
    
    # 检查配置文件
    if [ ! -f "$CONFIG_FILE" ]; then
        log_warning "配置文件不存在: $CONFIG_FILE，使用默认配置"
        # 可以在这里创建默认配置
    fi
    
    # 运行评估
    run_evaluation
    
    # 生成报告
    generate_report
    
    log_info "========================================"
    log_success "评估流程完成！"
    log_info "结果保存在: $OUTPUT_DIR"
    log_info "========================================"
}

# 运行主函数
main "$@"