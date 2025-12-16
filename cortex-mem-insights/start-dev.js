import { spawn } from 'child_process';

console.log('🚀 启动 cortex-mem-insights 开发服务...\n');

// 启动 API 服务器
console.log('📡 启动 API 服务器 (localhost:15173)...');
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

// 启动 Vite 开发服务器
console.log('\n🌐 启动 Vite 开发服务器 (localhost:5173)...');
const devProcess = spawn('bun', ['run', 'dev'], {
	stdio: 'inherit',
	shell: true,
	cwd: process.cwd()
});

devProcess.on('error', (error) => {
	console.error('❌ Vite 开发服务器启动失败:', error);
});

console.log(`\n✅ cortex-mem-insights 开发服务已启动！`);
console.log(`📊 访问 http://localhost:5173 查看 Web 界面`);
console.log(`🔌 API 服务运行在 http://localhost:15173`);
console.log(`\n按 Ctrl+C 停止所有服务\n`);

// 处理退出信号
process.on('SIGINT', () => {
	console.log('\n\n🛑 收到退出信号，关闭所有服务...');
	apiProcess.kill('SIGINT');
	devProcess.kill('SIGINT');
	process.exit(0);
});

process.on('SIGTERM', () => {
	console.log('\n\n🛑 收到终止信号，关闭所有服务...');
	apiProcess.kill('SIGTERM');
	devProcess.kill('SIGTERM');
	process.exit(0);
});

// 监听进程退出
apiProcess.on('exit', (code) => {
	console.log(`\n⚠️  API 服务器退出，代码: ${code}`);
	if (code !== 0) {
		console.log('正在关闭 Vite 开发服务器...');
		devProcess.kill('SIGTERM');
		process.exit(code);
	}
});

devProcess.on('exit', (code) => {
	console.log(`\n⚠️  Vite 开发服务器退出，代码: ${code}`);
	if (code !== 0) {
		console.log('正在关闭 API 服务器...');
		apiProcess.kill('SIGTERM');
		process.exit(code);
	}
});
