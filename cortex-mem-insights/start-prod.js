import { app } from './src/server/index.js';
import { staticPlugin } from '@elysiajs/static';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

console.log('🚀 启动 cortex-mem-insights 生产服务...\n');

const PORT = process.env.PORT || 15173;
const HOST = process.env.HOST || '0.0.0.0';

// 配置静态文件托管 - 需要在其他路由之后注册
const buildPath = path.join(__dirname, 'build');
console.log('📁 静态文件目录:', buildPath);

// 添加SPA fallback - 所有非API路由返回index.html
app.get('*', async ({ path: reqPath, set }) => {
	// 如果是API路由,跳过让API处理
	if (reqPath.startsWith('/api') || reqPath.startsWith('/health')) {
		return;
	}
	
	// 尝试作为静态文件处理
	const filePath = path.join(buildPath, reqPath);
	const file = Bun.file(filePath);
	
	if (await file.exists()) {
		// 设置正确的Content-Type
		const ext = path.extname(filePath);
		const contentTypes = {
			'.html': 'text/html',
			'.js': 'application/javascript',
			'.css': 'text/css',
			'.json': 'application/json',
			'.png': 'image/png',
			'.jpg': 'image/jpeg',
			'.svg': 'image/svg+xml',
			'.ico': 'image/x-icon'
		};
		set.headers['Content-Type'] = contentTypes[ext] || 'application/octet-stream';
		return file;
	}
	
	// 其他路由返回index.html (SPA fallback)
	set.headers['Content-Type'] = 'text/html';
	return Bun.file(path.join(buildPath, 'index.html'));
});

app.listen(PORT, () => {
	console.log(`\n✅ cortex-mem-insights 生产服务已启动！`);
	console.log(`📊 访问 http://localhost:${PORT} 查看 Web 界面`);
	console.log(`🔌 API 服务运行在 http://localhost:${PORT}/api`);
	console.log(`\n按 Ctrl+C 停止服务\n`);
});

// 处理退出信号
process.on('SIGINT', () => {
	console.log('\n\n🛑 收到退出信号，关闭服务...');
	process.exit(0);
});

process.on('SIGTERM', () => {
	console.log('\n\n🛑 收到终止信号，关闭服务...');
	process.exit(0);
});
