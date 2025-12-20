// 国际化迁移脚本
// 这个脚本帮助将现有页面的硬编码文本替换为国际化key

const fs = require('fs');
const path = require('path');

// 需要替换的文本映射（中文 -> 国际化key）
const replacements = {
  // 通用
  '仪表盘': 'dashboard.title',
  '欢迎使用 Cortex Memory 洞察': 'dashboard.welcome',
  '监控和分析 Cortex Memory 系统的运行状态': 'dashboard.welcome',
  '总记忆数': 'dashboard.totalMemories',
  '优化次数': 'dashboard.optimizationCount',
  '平均质量': 'dashboard.averageQuality',
  '质量分布': 'dashboard.qualityDistribution',
  '高/中/低': 'dashboard.highMediumLow',
  '系统状态': 'dashboard.systemStatus',
  '最近记忆': 'dashboard.recentMemories',
  '查看全部': 'dashboard.viewAll',
  '暂无记忆': 'dashboard.noMemories',
  '检测中': 'dashboard.detecting',
  '健康': 'dashboard.healthy',
  '不健康': 'dashboard.unhealthy',
  'Cortex Memory 服务': 'dashboard.cortexMemService',
  'LLM 服务': 'dashboard.llmService',
  '向量存储': 'dashboard.vectorStore',
  '延迟': 'dashboard.latency',
  '版本': 'dashboard.version',
  '最后检查': 'dashboard.lastCheck',
  
  // 记忆页面
  '记忆管理': 'memories.title',
  '搜索记忆...': 'memories.searchPlaceholder',
  '类型筛选': 'memories.typeFilter',
  '所有类型': 'memories.allTypes',
  '排序方式': 'memories.sortBy',
  '创建时间': 'memories.createdAt',
  '重要性': 'memories.importance',
  '升序': 'memories.ascending',
  '降序': 'memories.descending',
  '全选': 'memories.selectAll',
  '批量操作': 'memories.batchOperations',
  '删除选中': 'memories.deleteSelected',
  '导出选中': 'memories.exportSelected',
  '未找到记忆': 'memories.noMemoriesFound',
  '加载记忆中...': 'memories.loadingMemories',
  '记忆详情': 'memories.memoryDetails',
  '内容': 'memories.content',
  '类型': 'memories.type',
  '用户ID': 'memories.userId',
  '代理ID': 'memories.agentId',
  '创建时间': 'memories.created',
  '更新时间': 'memories.updated',
  '操作': 'memories.actions',
  '确认删除': 'memories.confirmDelete',
  '确定要删除此记忆吗？': 'memories.deleteMemoryConfirm',
  '确定要删除 {count} 条记忆吗？': 'memories.deleteMemoriesConfirm',
  '记忆删除成功': 'memories.memoryDeleted',
  '{count} 条记忆删除成功': 'memories.memoriesDeleted',
  '导出格式': 'memories.exportFormat',
  'JSON': 'memories.json',
  'CSV': 'memories.csv',
  '文本': 'memories.txt',
  
  // 分析页面
  '统计分析': 'analytics.title',
  '概览': 'analytics.summary',
  '总记忆数': 'analytics.totalMemories',
  '活跃用户': 'analytics.activeUsers',
  '平均质量': 'analytics.averageQuality',
  '基于重要性评分': 'analytics.basedOnImportance',
  '质量评分分布': 'analytics.qualityDistribution',
  '用户活跃度': 'analytics.userActivity',
  '记忆数量': 'analytics.memoryCount',
  '平均重要性': 'analytics.avgImportance',
  '百分比': 'analytics.percentage',
  '时间趋势': 'analytics.timeTrend',
  '最近7天': 'analytics.last7Days',
  '最近30天': 'analytics.last30Days',
  '新增记忆趋势': 'analytics.newMemoriesTrend',
  '暂无数据': 'analytics.noData',
  '加载分析数据...': 'analytics.loadingAnalytics',
  
  // 监控页面
  '系统监控': 'monitor.title',
  '系统指标': 'monitor.systemMetrics',
  '内存使用率': 'monitor.memoryUsage',
  'CPU 使用率': 'monitor.cpuUsage',
  '磁盘使用率': 'monitor.diskUsage',
  '活跃连接数': 'monitor.activeConnections',
  '请求数量': 'monitor.requestCount',
  '错误率': 'monitor.errorRate',
  '响应时间': 'monitor.responseTime',
  '告警': 'monitor.alerts',
  '无告警': 'monitor.noAlerts',
  '严重': 'monitor.critical',
  '警告': 'monitor.warning',
  '信息': 'monitor.info',
  '健康': 'monitor.healthy',
  '阈值': 'monitor.threshold',
  '当前值': 'monitor.current',
  '状态': 'monitor.status',
  '最后更新': 'monitor.lastUpdated',
  
  // 优化页面
  '记忆优化': 'optimization.title',
  '运行优化': 'optimization.runOptimization',
  '优化历史': 'optimization.optimizationHistory',
  '状态': 'optimization.status',
  '等待中': 'optimization.pending',
  '运行中': 'optimization.running',
  '已完成': 'optimization.completed',
  '失败': 'optimization.failed',
  '总记忆数': 'optimization.totalMemories',
  '已处理': 'optimization.processed',
  '去重': 'optimization.deduplicated',
  '合并': 'optimization.merged',
  '增强': 'optimization.enhanced',
  '错误': 'optimization.errors',
  '开始时间': 'optimization.startTime',
  '结束时间': 'optimization.endTime',
  '持续时间': 'optimization.duration',
  '操作': 'optimization.actions',
  '查看详情': 'optimization.viewDetails',
  '取消': 'optimization.cancel',
  '试运行': 'optimization.dryRun',
  '详细模式': 'optimization.verbose',
  '开始优化': 'optimization.startOptimization',
  '优化已开始': 'optimization.optimizationStarted',
  '暂无优化历史': 'optimization.noHistory'
};

// 需要处理的文件
const filesToProcess = [
  'src/routes/+page.svelte',
  'src/routes/memories/+page.svelte',
  'src/routes/analytics/+page.svelte',
  'src/routes/monitor/+page.svelte',
  'src/routes/optimization/+page.svelte'
];

function processFile(filePath) {
  const fullPath = path.join(__dirname, '..', filePath);
  
  if (!fs.existsSync(fullPath)) {
    console.log(`文件不存在: ${filePath}`);
    return;
  }
  
  let content = fs.readFileSync(fullPath, 'utf8');
  let modified = false;
  
  // 首先添加国际化导入（如果还没有）
  if (!content.includes("from '$lib/i18n'")) {
    const importMatch = content.match(/import.*from.*['"][^'"]+['"]/);
    if (importMatch) {
      const importStatement = importMatch[0];
      if (!importStatement.includes("$lib/i18n")) {
        content = content.replace(importStatement, `${importStatement}\n\timport { t, format } from '$lib/i18n';`);
        modified = true;
      }
    }
  }
  
  // 替换文本
  for (const [chinese, key] of Object.entries(replacements)) {
    // 替换纯文本（不在花括号内）
    const regex = new RegExp(`>\\s*${chinese.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}\\s*<`, 'g');
    if (regex.test(content)) {
      content = content.replace(regex, `>{$t('${key}')}<`);
      modified = true;
      console.log(`在 ${filePath} 中替换: ${chinese} -> {$t('${key}')}`);
    }
    
    // 替换带参数的文本
    const paramRegex = new RegExp(`\\{${chinese.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}\\}`, 'g');
    if (paramRegex.test(content)) {
      content = content.replace(paramRegex, `{$t('${key}')}`);
      modified = true;
      console.log(`在 ${filePath} 中替换（带参数）: {${chinese}} -> {$t('${key}')}`);
    }
  }
  
  if (modified) {
    fs.writeFileSync(fullPath, content, 'utf8');
    console.log(`已更新: ${filePath}`);
  } else {
    console.log(`无需更新: ${filePath}`);
  }
}

// 处理所有文件
console.log('开始国际化迁移...');
filesToProcess.forEach(processFile);
console.log('迁移完成！');

// 使用说明
console.log('\n使用说明:');
console.log('1. 运行此脚本: node scripts/i18n-migration.js');
console.log('2. 手动检查替换结果，确保没有误替换');
console.log('3. 对于复杂的替换（如带参数的文本），可能需要手动调整');
console.log('4. 测试所有页面的国际化功能');
