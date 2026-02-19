# Cortex Memory Insights - 可观测性仪表板

`cortex-mem-insights` 是一个基于 Svelte 构建的 Web 仪表板，提供对 Cortex Memory 系统的可视化监控和管理界面。

## 🌟 功能特性

### 📊 系统仪表板
- 📈 **实时监控**: 实时显示系统状态、会话数量、消息计数等关键指标
- 📉 **性能图表**: 可视化搜索性能、提取速度、响应时间等关键性能指标
- 🔍 **系统健康**: 监控 LLM 服务、向量数据库、文件系统等依赖组件的健康状态

### 💾 记忆管理
- 📂 **浏览结构**: 直观浏览 Cortex 文件系统结构，包括用户、代理、会话和资源维度
- 📝 **查看内容**: 分层查看记忆内容（L0 抽象、L1 概览、L3 完整内容）
- 🏷️ **标签管理**: 查看和管理记忆标签、分类和重要程度

### 🔍 智能搜索
- 🎯 **高级搜索**: 支持多维度、时间范围、标签过滤的高级搜索功能
- 📊 **结果可视化**: 搜索结果以卡片、列表或时间轴形式展示
- 💾 **搜索历史**: 保存和管理搜索历史，支持收藏常用查询

### 📱 响应式设计
- 🖥️ **桌面适配**: 优化的桌面端体验，支持宽屏显示
- 📱 **移动友好**: 响应式布局，适配平板和手机设备
- 🌙 **主题切换**: 支持明暗主题自动切换

## 🚀 快速开始

### 开发环境要求

- Node.js 18+ 或 Bun
- 现代浏览器（Chrome 90+, Firefox 88+, Safari 14+）

### 安装与运行

```bash
# 进入目录
cd cortex-mem/cortex-mem-insights

# 安装依赖
bun install  # 或 npm install

# 启动开发服务器
bun run dev  # 或 npm run dev

# 构建生产版本
bun run build  # 或 npm run build

# 预览生产构建
bun run preview  # 或 npm run preview
```

访问 http://localhost:5173 开始使用。

### 配置连接

编辑 `src/lib/api.ts` 中的 API_BASE 常量指向您的 Cortex Memory Service：

```typescript
const API_BASE = 'http://localhost:8080/api/v2';
```

## 📖 使用指南

### 仪表板概览

主仪表板提供：
- **系统状态**: 实时显示服务状态和数据目录使用情况
- **活动图表**: 展示最近 24 小时的消息活动和搜索频率
- **快速操作**: 常用功能的快捷访问按钮

### 浏览记忆

1. 点击左侧导航栏的 "Memories"
2. 选择维度（用户、代理、会话、资源）
3. 浏览记忆结构，点击展开查看内容
4. 使用顶部按钮切换抽象/概览/完整内容视图

### 高级搜索

1. 导航到 "Search" 页面
2. 输入搜索查询
3. 应用过滤器：
   - 范围：会话、用户、代理、全局
   - 时间范围：自定义日期区间
   - 最低相关性分数
4. 查看结果并点击查看详细内容

### 监控系统

1. 仪表板自动显示系统健康状态
2. 绿色指示器表示正常，黄色警告，红色错误
3. 点击指示器查看详细错误信息和解决方案

## 🛠️ 技术架构

### 前端技术栈
- **Svelte 5**: 响应式 UI 框架，使用最新的 runes API
- **Vite**: 快速构建工具和开发服务器
- **TypeScript**: 类型安全的 JavaScript 超集
- **Svelte Routing**: 客户端路由解决方案

### 项目结构
```
src/
├── lib/
│   ├── api.ts          # API 客户端
│   ├── types.ts        # TypeScript 类型定义
│   ├── components/     # 可复用组件
│   ├── pages/          # 页面组件
│   └── stores/         # 状态管理
├── routes/            # 路由定义
├── app.html           # HTML 模板
└── main.ts            # 应用入口点
```

### API 集成

仪表板通过 REST API 与 Cortex Memory Service 通信：
- 获取会话列表和详情
- 搜索和浏览记忆内容
- 检索系统状态和健康信息
- 执行记忆提取和索引操作

## 🔧 配置选项

### 环境变量

创建 `.env` 文件配置环境变量：

```bash
# API 服务器地址
VITE_API_BASE_URL=http://localhost:8080/api/v2

# 应用标题
VITE_APP_TITLE=Cortex Memory Insights

# 主题设置
VITE_DEFAULT_THEME=dark

# 自动刷新间隔（毫秒）
VITE_REFRESH_INTERVAL=5000
```

### 自定义样式

编辑 `src/app.css` 修改样式变量：

```css
:root {
  --primary-color: #6366f1;
  --secondary-color: #22d3ee;
  --background-color: #0f172a;
  --text-color: #e2e8f0;
  --border-color: #334155;
}
```

## 🧪 开发指南

### 添加新页面

1. 在 `src/lib/pages/` 创建新页面组件
2. 在 `src/App.svelte` 添加路由和导航
3. 更新类型定义（如需要）

### 自定义组件

可复用组件存放在 `src/lib/components/`：
- `MemoryCard.svelte` - 记忆卡片显示
- `SearchBox.svelte` - 搜索输入框
- `StatusIndicator.svelte` - 状态指示器
- `Chart.svelte` - 通用图表组件

### 状态管理

使用 Svelte 5 的 runes 进行状态管理：
- `$state` - 响应式状态
- `$derived` - 派生状态
- `$effect` - 副作用处理

## 📊 性能优化

### 代码分割
- 路由级别代码自动分割
- 按需加载大型组件
- 延迟加载图表库

### 缓存策略
- API 响应缓存（5分钟）
- 静态资源长期缓存
- 浏览器缓存优化

### 优化技巧
```typescript
// 使用 $derived 避免不必要的计算
const filteredMemories = $derived(
  memories.filter(m => m.score > minScore)
);

// 延迟加载大型数据
async function loadDetailedView(uri: string) {
  const content = await api.getMemory(uri);
  detailedContent.set(content);
}
```

## 🚨 常见问题

### 连接失败

**问题**: 无法连接到 Cortex Memory Service
**解决**: 
1. 检查服务是否运行（`http://localhost:8080/health`）
2. 验证 CORS 设置
3. 检查防火墙配置

### 数据显示错误

**问题**: 记忆内容显示异常
**解决**:
1. 刷新页面重新加载数据
2. 检查服务端日志
3. 验证数据格式兼容性

### 性能问题

**问题**: 仪表板响应缓慢
**解决**:
1. 减少同时显示的数据量
2. 增加搜索过滤条件
3. 检查网络延迟

## 🛣️ 路线图

### 短期计划
- [ ] 实时 WebSocket 连接
- [ ] 记忆关系图谱可视化
- [ ] 批量操作功能
- [ ] 导出/导入工具

### 长期计划
- [ ] 多租户支持
- [ ] 自定义仪表板
- [ ] 插件系统
- [ ] 移动应用版本

## 🤝 贡献指南

欢迎贡献！请遵循以下步骤：

1. Fork 项目
2. 创建功能分支
3. 提交更改
4. 创建 Pull Request

代码风格：
- 使用 2 空格缩进
- 组件命名使用 PascalCase
- 文件名使用 kebab-case
- 提交消息使用 Conventional Commits

## 📄 许可证

MIT 许可证 - 详见 [LICENSE](../../LICENSE) 文件

## 🔗 相关资源

- [Cortex Memory 主文档](../README.md)
- [Cortex Memory Service](../cortex-mem-service/README.md)
- [Cortex Memory 核心](../cortex-mem-core/README.md)
- [Svelte 文档](https://svelte.dev/docs)
- [Vite 文档](https://vitejs.dev/)

---

**Built with ❤️ using Svelte and TypeScript**