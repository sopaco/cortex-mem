// 测试脚本用于验证Mock数据功能
import { cortexMemService } from './src/server/integrations/cortex-mem.js';

async function testMockData() {
  console.log('🧪 测试Mock数据功能...\n');
  
  try {
    // 测试健康检查
    console.log('1. 测试健康检查...');
    const health = await cortexMemService.healthCheck();
    console.log('健康检查结果:', health);
    
    // 测试获取记忆列表
    console.log('\n2. 测试获取记忆列表...');
    const memories = await cortexMemService.listMemories({});
    console.log('记忆列表结果:', {
      total: memories.total,
      memories: memories.memories.length
    });
    
    if (memories.memories.length > 0) {
      console.log('第一条记忆:', {
        id: memories.memories[0].id,
        content: memories.memories[0].content.substring(0, 50) + '...',
        type: memories.memories[0].metadata.memory_type
      });
    }
    
    // 测试搜索功能
    console.log('\n3. 测试搜索功能...');
    const searchResults = await cortexMemService.searchMemories('SkyronJ', { limit: 2 });
    console.log('搜索结果:', {
      total: searchResults.total,
      results: searchResults.results.length
    });
    
    if (searchResults.results.length > 0) {
      console.log('第一个搜索结果:', {
        id: searchResults.results[0].memory.id,
        score: searchResults.results[0].score,
        content: searchResults.results[0].memory.content.substring(0, 50) + '...'
      });
    }
    
    // 测试过滤功能
    console.log('\n4. 测试过滤功能...');
    const filteredMemories = await cortexMemService.listMemories({ 
      user_id: 'SkyronJ',
      memory_type: 'Personal'
    });
    console.log('过滤结果:', {
      total: filteredMemories.total,
      memories: filteredMemories.memories.length
    });
    
    console.log('\n✅ 所有测试通过！Mock数据功能正常工作。');
    
  } catch (error) {
    console.error('❌ 测试失败:', error);
    process.exit(1);
  }
}

testMockData();