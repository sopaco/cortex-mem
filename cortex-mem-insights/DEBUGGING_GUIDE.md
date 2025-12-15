# cortex-mem-insights 调试指南

## 当前问题

**症状**：记忆浏览器页面显示"未找到记忆记录"，但控制台显示有数据

**可能原因**：
1. API 数据加载失败
2. 数据转换错误
3. 过滤逻辑问题
4. 响应式变量未更新
5. Vite 代理配置问题

## 调试步骤

### 1. 检查 API 连接

**方法**：使用 debug-api.html 测试 API 连接

**步骤**：
1. 打开 `debug-api.html` 在浏览器中
2. 点击 "Test Direct API Call" 测试直接 API 连接
3. 点击 "Test Through Vite Proxy" 测试代理连接
4. 点击 "Check Network" 检查网络状态

**预期结果**：
- 直接 API 调用应该返回数据
- 代理 API 调用应该返回相同数据
- 网络检查应该显示两个服务都健康

### 2. 检查控制台日志

**方法**：打开浏览器开发者工具 (F12) → 控制台

**查看**：
- `API Base URL:` - 应该显示空字符串（开发模式）
- `加载记忆 - 开始` - 应该出现
- `加载记忆 - API响应:` - 应该显示 API 响应
- `加载记忆 - 记忆数量:` - 应该显示 42
- `加载记忆 - 转换后的记忆数量:` - 应该显示 42
- `加载记忆 - 最终memories数组长度:` - 应该显示 42
- `filteredMemories 计算 - memories长度:` - 应该显示 42
- `filteredMemories 计算 - 最终结果长度:` - 应该显示 42

**常见问题**：
- 如果 `API响应:` 未出现 → API 调用失败
- 如果 `记忆数量:` 为 0 → API 返回空数据
- 如果 `转换后的记忆数量:` 为 0 → 数据转换失败
- 如果 `最终memories数组长度:` 为 0 → 赋值失败
- 如果 `filteredMemories 计算 - memories长度:` 为 0 → 响应式触发失败

### 3. 检查网络请求

**方法**：打开浏览器开发者工具 (F12) → 网络

**查看**：
- 请求到 `/api/memories`
- 请求状态码应该是 200
- 响应应该包含 JSON 数据
- 响应头应该包含 `Content-Type: application/json`

**常见问题**：
- 404 错误 → 请求 URL 错误
- CORS 错误 → 代理配置问题
- 500 错误 → 服务器错误
- 空响应 → API 服务问题

### 4. 检查 Vite 代理

**方法**：检查 `vite.config.ts` 配置

**当前配置**：
```javascript
server: {
  proxy: {
    '/api': {
      target: 'http://localhost:3001',
      changeOrigin: true,
      secure: false
    },
    '/health': {
      target: 'http://localhost:3001',
      changeOrigin: true,
      secure: false
    }
  }
}
```

**验证方法**：
1. 确保 API 服务运行在 `http://localhost:3001`
2. 确保前端服务运行在 `http://localhost:5173` (或其他端口)
3. 测试代理是否工作：`curl http://localhost:5173/api/memories`

### 5. 检查数据转换

**方法**：查看 `src/routes/memories/+page.svelte` 中的转换逻辑

**关键代码**：
```typescript
// 转换API响应到前端数据结构
const transformedMemories = response.memories.map((memory: any) => {
  // 处理编码问题：尝试修复乱码
  let content = memory.content;
  
  // 从custom字段获取重要性分数
  let importance = 0.7;
  
  return {
    id: memory.id,
    content: content,
    type: memory.metadata.memory_type.toLowerCase(),
    importance: importance,
    userId: memory.metadata.user_id,
    agentId: memory.metadata.agent_id,
    createdAt: memory.created_at,
    updatedAt: memory.updated_at
  };
});

// 确保memories数组被正确赋值
memories = transformedMemories;
```

**常见问题**：
- `memory.metadata.memory_type` 为空 → `toLowerCase()` 失败
- `memory.content` 为空 → 内容显示问题
- `memory.created_at` 格式错误 → 日期解析失败

## 常见问题解决方案

### 问题 1：API 调用失败

**症状**：控制台显示 `API request failed: /api/memories`

**解决方案**：
1. 检查 API 服务是否运行
2. 检查 Vite 代理配置
3. 检查网络连接
4. 尝试直接调用 API：`curl http://localhost:3001/api/memories`

**修复方法**：
```bash
# 重启 API 服务
cd cortex-mem-insights
bun run start-api.js

# 检查 API 健康状态
curl http://localhost:3001/health
```

### 问题 2：数据转换失败

**症状**：控制台显示 `记忆数量: 42` 但 `转换后的记忆数量: 0`

**解决方案**：
1. 检查 API 响应结构
2. 确保所有字段存在
3. 添加错误处理

**修复方法**：
```typescript
// 添加错误处理到数据转换
const transformedMemories = response.memories.map((memory: any) => {
  try {
    if (!memory || !memory.id || !memory.content || !memory.metadata) {
      console.warn('无效的记忆数据:', memory);
      return null;
    }
    
    return {
      id: memory.id,
      content: memory.content || '',
      type: (memory.metadata.memory_type || '').toLowerCase(),
      importance: 0.7,
      userId: memory.metadata.user_id,
      agentId: memory.metadata.agent_id,
      createdAt: memory.created_at,
      updatedAt: memory.updated_at
    };
  } catch (error) {
    console.error('转换记忆失败:', memory, error);
    return null;
  }
}).filter(m => m !== null); // 过滤无效记忆
```

### 问题 3：响应式变量未更新

**症状**：控制台显示 `最终memories数组长度: 42` 但 `filteredMemories 计算 - memories长度: 0`

**解决方案**：
1. 检查响应式声明
2. 确保变量被正确赋值
3. 检查 Svelte 响应式触发

**修复方法**：
```typescript
// 确保响应式触发
memories = transformedMemories;
// 强制触发响应式更新
memories = [...transformedMemories];
```

### 问题 4：过滤逻辑问题

**症状**：控制台显示 `filteredMemories 计算 - memories长度: 42` 但 `最终结果长度: 0`

**解决方案**：
1. 检查过滤条件
2. 检查搜索查询
3. 检查类型过滤

**修复方法**：
```typescript
console.log('过滤条件:', {
  searchQuery,
  selectedType,
  memoriesLength: memories.length
});

// 检查每个过滤步骤
console.log('过滤前:', result.length);

// 搜索过滤
if (searchQuery) {
  const beforeSearch = result.length;
  result = result.filter(memory => 
    memory.content.toLowerCase().includes(query) ||
    memory.id.toLowerCase().includes(query)
  );
  console.log('搜索过滤后:', result.length, '(从', beforeSearch, ')');
}

// 类型过滤
if (selectedType !== 'all') {
  const beforeType = result.length;
  result = result.filter(memory => memory.type === selectedType);
  console.log('类型过滤后:', result.length, '(从', beforeType, ')');
}
```

### 问题 5：Vite 代理配置错误

**症状**：直接 API 调用工作，但代理调用失败

**解决方案**：
1. 检查 Vite 配置
2. 确保代理目标正确
3. 检查 CORS 设置

**修复方法**：
```javascript
// 在 vite.config.ts 中
server: {
  proxy: {
    '/api': {
      target: 'http://localhost:3001',  // 确保端口正确
      changeOrigin: true,
      secure: false,
      rewrite: (path) => path.replace(/^\/api/, '/api')  // 确保路径正确
    }
  }
}
```

## 高级调试技术

### 1. 手动测试 API

```bash
# 测试健康检查
curl -v http://localhost:3001/health

# 测试记忆列表
curl -v http://localhost:3001/api/memories

# 测试搜索
curl -v -X POST http://localhost:3001/api/memories/search \
  -H "Content-Type: application/json" \
  -d '{"query":"SkyronJ"}'
```

### 2. 手动测试代理

```bash
# 测试通过 Vite 代理
curl -v http://localhost:5173/api/memories

# 检查响应头
curl -I http://localhost:5173/api/memories
```

### 3. 网络抓包

使用浏览器开发者工具：
1. 打开网络面板
2. 保持请求日志
3. 查看每个请求的时间线
4. 检查响应内容

### 4. 断点调试

在 Chrome 中：
1. 打开源代码面板
2. 找到 `src/routes/memories/+page.svelte`
3. 在 `loadMemories` 函数中添加断点
4. 刷新页面
5. 逐步执行代码

## 修复日志

### 2025-12-15 修复

**问题**：记忆浏览器显示"未找到记忆记录"尽管 API 返回数据

**调试步骤**：
1. 添加详细控制台日志到 `loadMemories` 函数
2. 添加详细控制台日志到 `filteredMemories` 计算
3. 测试 API 连接
4. 验证数据转换

**发现**：
- API 返回 42 条记忆
- 数据转换成功
- `memories` 数组被正确赋值
- `filteredMemories` 计算被触发

**可能根本原因**：
- 响应式变量未正确触发
- 过滤逻辑过于严格
- 类型匹配问题

**建议修复**：
1. 检查 `selectedType` 默认值
2. 检查类型过滤逻辑
3. 添加更多调试日志

## 解决方案

### 方案 1：检查默认过滤器

```svelte
<!-- 在 +page.svelte 中 -->
<script>
  // 确保默认类型是 'all'
  let selectedType = 'all';
  
  // 添加调试日志
  console.log('初始 selectedType:', selectedType);
</script>
```

### 方案 2：简化过滤逻辑

```typescript
// 简化过滤逻辑
$: {
  console.log('计算 filteredMemories...');
  
  let result = [...memories];
  console.log('初始结果:', result.length);
  
  // 仅应用类型过滤（暂时禁用搜索以简化调试）
  if (selectedType !== 'all') {
    result = result.filter(memory => memory.type === selectedType);
    console.log('类型过滤后:', result.length);
  }
  
  filteredMemories = result;
  console.log('最终 filteredMemories:', filteredMemories.length);
}
```

### 方案 3：强制响应式更新

```typescript
// 在 loadMemories 结束时
memories = transformedMemories;
// 强制触发响应式更新
setTimeout(() => {
  memories = [...transformedMemories];
}, 100);
```

### 方案 4：检查类型匹配

```typescript
// 检查类型是否匹配
const testMemory = transformedMemories[0];
if (testMemory) {
  console.log('测试记忆类型:', testMemory.type);
  console.log('选中类型:', selectedType);
  console.log('是否匹配:', testMemory.type === selectedType);
}
```

## 最终解决方案

经过上述调试步骤，最可能的问题是：

1. **`selectedType` 不是 'all'** → 导致所有记忆被过滤掉
2. **类型不匹配** → 记忆类型与过滤器不匹配
3. **响应式未触发** → `filteredMemories` 未重新计算

### 修复步骤：

1. **检查 `selectedType` 初始值**：
   ```svelte
   <script>
     let selectedType = 'all'; // 确保是 'all'
   </script>
   ```

2. **检查类型转换**：
   ```typescript
   type: memory.metadata.memory_type.toLowerCase(), // 确保转换正确
   ```

3. **检查过滤逻辑**：
   ```typescript
   if (selectedType !== 'all') {
     result = result.filter(memory => memory.type === selectedType);
   }
   ```

4. **添加调试日志**：
   ```typescript
   console.log('过滤前:', memories.length);
   console.log('选中类型:', selectedType);
   console.log('第一个记忆类型:', memories[0]?.type);
   ```

如果问题仍然存在，请检查：
- 浏览器控制台是否有错误
- 网络请求是否成功
- 响应数据结构是否正确
- 响应式变量是否被正确更新

## 联系支持

如果经过上述步骤仍无法解决问题，请提供：
1. 完整的控制台日志
2. 网络请求截图
3. 浏览器和操作系统版本
4. 具体的错误信息

我们将尽快帮助您解决问题！

---

> "系统调试需要耐心和方法。通过逐步排除可能性，我们最终能找到问题的根源。"

**状态**：调试中 🔍