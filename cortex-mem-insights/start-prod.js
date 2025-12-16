import { handler } from './build/handler.js';
import { spawn } from 'child_process';

console.log('🚀 启动 cortex-mem-insights 生产服务...\n');

// 启动 API 服务器 (bun service)
console.log('📡 启动 API 服务器 (localhost:3001)...');
const apiProcess = spawn('bun', ['run', 'start-api.js'], {
	stdio: 'inherit',
	shell: true,
	cwd: process.cwd()
});

apiProcess.on('error', (error) => {
	console.error('❌ API 服务器启动失败:', error);
});

// 等待 API 服务器启动
await new Promise((resolve) => setTimeout(resolve, 2000));

// 启动 SvelteKit 生产服务器
console.log('\n🌐 启动 Web 服务器 (localhost:3000)...');
const PORT = process.env.PORT || 15173;
const HOST = process.env.HOST || '0.0.0.0';

const server = Bun.serve({
	port: PORT,
	hostname: HOST,
	fetch: handler
});

console.log(`\n✅ cortex-mem-insights 生产服务已启动！`);
console.log(`📊 访问 http://localhost:${PORT} 查看 Web 界面`);
console.log(`🔌 API 服务运行在 http://localhost:3001`);
console.log(`\n按 Ctrl+C 停止所有服务\n`);

// 处理退出信号
process.on('SIGINT', () => {
	console.log('\n\n🛑 收到退出信号，关闭所有服务...');
	server.stop();
	apiProcess.kill('SIGINT');
	process.exit(0);
});

process.on('SIGTERM', () => {
	console.log('\n\n🛑 收到终止信号，关闭所有服务...');
	server.stop();
	apiProcess.kill('SIGTERM');
	process.exit(0);
});

// 监听 API 进程退出
apiProcess.on('exit', (code) => {
	console.log(`\n⚠️  API 服务器退出，代码: ${code}`);
	if (code !== 0) {
		console.log('正在关闭 Web 服务器...');
		server.stop();
		process.exit(code);
	}
});
