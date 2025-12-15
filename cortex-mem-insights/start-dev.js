import { spawn } from 'child_process';

console.log('启动 cortex-mem-insights 开发服务器...');

const devProcess = spawn('bun', ['run', 'dev'], {
  stdio: 'inherit',
  shell: true,
  cwd: process.cwd()
});

devProcess.on('error', (error) => {
  console.error('启动失败:', error);
});

devProcess.on('exit', (code) => {
  console.log(`开发服务器退出，代码: ${code}`);
});

// 处理退出信号
process.on('SIGINT', () => {
  console.log('\n收到退出信号，关闭开发服务器...');
  devProcess.kill('SIGINT');
  process.exit(0);
});