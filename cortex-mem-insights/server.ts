/**
 * Cortex Memory Insights - Standalone Server
 *
 * 这个文件是打包成可执行文件的入口点
 * 使用Bun的静态文件服务器功能 + API代理
 *
 * Features:
 * - 自动serve Vite构建的dist/目录
 * - API代理到cortex-mem-service
 * - 内嵌HTTP服务器
 * - 自动打开浏览器
 * - 支持自定义端口
 */

import { spawn } from 'child_process';
import { existsSync } from 'fs';
import { join } from 'path';

const DEFAULT_PORT = 8159;
const HOST = '127.0.0.1';
const API_TARGET = process.env.API_TARGET || 'http://localhost:8085'; // cortex-mem-service地址

// 版本信息（可通过--define注入）
declare const VERSION: string | undefined;
declare const BUILD_TIME: string | undefined;

const version = typeof VERSION !== 'undefined' ? VERSION : 'dev';
const buildTime = typeof BUILD_TIME !== 'undefined' ? BUILD_TIME : new Date().toISOString();

/**
 * 自动打开浏览器
 */
function openBrowser(url: string): void {
	const platform = process.platform;
	let command: string;

	if (platform === 'darwin') {
		command = 'open';
	} else if (platform === 'win32') {
		command = 'start';
	} else {
		// Linux and others
		command = 'xdg-open';
	}

	try {
		spawn(command, [url], {
			detached: true,
			stdio: 'ignore'
		}).unref();
		console.log(`🌐 Opening browser at ${url}...`);
	} catch (error) {
		console.warn(`⚠️  Could not open browser automatically: ${error}`);
		console.log(`📝 Please open ${url} manually`);
	}
}

/**
 * 解析命令行参数
 */
function parseArgs(): { port: number; noBrowser: boolean; help: boolean; apiTarget: string } {
	const args = process.argv.slice(2);

	// 帮助信息
	if (args.includes('--help') || args.includes('-h')) {
		return { port: DEFAULT_PORT, noBrowser: false, help: true, apiTarget: API_TARGET };
	}

	// 端口
	let port = DEFAULT_PORT;
	const portIndex = args.findIndex((arg) => arg === '--port' || arg === '-p');
	if (portIndex >= 0 && args[portIndex + 1]) {
		const parsedPort = parseInt(args[portIndex + 1]);
		if (!isNaN(parsedPort) && parsedPort > 0 && parsedPort < 65536) {
			port = parsedPort;
		}
	}

	// 环境变量端口
	if (process.env.PORT) {
		const envPort = parseInt(process.env.PORT);
		if (!isNaN(envPort) && envPort > 0 && envPort < 65536) {
			port = envPort;
		}
	}

	// API target
	let apiTarget = API_TARGET;
	const apiIndex = args.findIndex((arg) => arg === '--api-target' || arg === '--api');
	if (apiIndex >= 0 && args[apiIndex + 1]) {
		apiTarget = args[apiIndex + 1];
	}

	// 禁用自动打开浏览器
	const noBrowser = args.includes('--no-browser') || args.includes('--headless');

	return { port, noBrowser, help: false, apiTarget };
}

/**
 * 显示帮助信息
 */
function showHelp(): void {
	console.log(`
╔════════════════════════════════════════════════╗
║   Cortex Memory Insights v${version.padEnd(18)}    ║
║   Standalone Server                            ║
╚════════════════════════════════════════════════╝

Usage: cortex-mem-insights [options]

Options:
  -p, --port <number>       Port to run server on (default: ${DEFAULT_PORT})
  --api-target <url>        API target URL (default: ${API_TARGET})
  --no-browser              Don't open browser automatically
  --headless                Same as --no-browser
  -h, --help                Show this help message

Environment Variables:
  PORT                      Port to run server on
  API_TARGET                API target URL for proxying

Examples:
  cortex-mem-insights
  cortex-mem-insights --port 8080
  cortex-mem-insights --api-target http://localhost:8085
  cortex-mem-insights --no-browser
  PORT=8080 API_TARGET=http://api.example.com cortex-mem-insights

Build Information:
  Version: ${version}
  Build Time: ${buildTime}
`);
}

/**
 * 获取dist目录路径
 */
function getDistPath(): string {
	// 尝试多个可能的路径
	const possiblePaths = [
		join(import.meta.dir, 'dist'), // 开发模式
		join(import.meta.dir, '..', 'dist'), // 编译后可能的路径
		join(process.cwd(), 'dist') // 当前工作目录
	];

	for (const path of possiblePaths) {
		if (existsSync(path)) {
			return path;
		}
	}

	// 如果都不存在，返回第一个（会在后面报错）
	return possiblePaths[0];
}

/**
 * 获取MIME type
 */
function getMimeType(path: string): string {
	const ext = path.split('.').pop()?.toLowerCase();
	const mimeTypes: Record<string, string> = {
		html: 'text/html',
		css: 'text/css',
		js: 'application/javascript',
		json: 'application/json',
		png: 'image/png',
		jpg: 'image/jpeg',
		jpeg: 'image/jpeg',
		gif: 'image/gif',
		svg: 'image/svg+xml',
		ico: 'image/x-icon',
		woff: 'font/woff',
		woff2: 'font/woff2',
		ttf: 'font/ttf',
		eot: 'application/vnd.ms-fontobject'
	};
	return mimeTypes[ext || ''] || 'application/octet-stream';
}

/**
 * 代理请求到后端API
 */
async function proxyRequest(req: Request, apiTarget: string): Promise<Response> {
	const url = new URL(req.url);
	const targetUrl = `${apiTarget}${url.pathname}${url.search}`;

	try {
		// 复制请求头，但移除host
		const headers = new Headers(req.headers);
		headers.delete('host');

		// 转发请求
		const proxyReq = new Request(targetUrl, {
			method: req.method,
			headers: headers,
			body: req.method !== 'GET' && req.method !== 'HEAD' ? req.body : undefined
		});

		const response = await fetch(proxyReq);

		// 复制响应头
		const responseHeaders = new Headers(response.headers);
		// 添加CORS头（如果需要）
		responseHeaders.set('Access-Control-Allow-Origin', '*');

		return new Response(response.body, {
			status: response.status,
			statusText: response.statusText,
			headers: responseHeaders
		});
	} catch (error) {
		console.error(`❌ Proxy error for ${url.pathname}:`, error);
		return new Response(
			JSON.stringify({
				success: false,
				error: `Failed to connect to backend service at ${apiTarget}. Please ensure cortex-mem-service is running.`,
				timestamp: new Date().toISOString()
			}),
			{
				status: 503,
				headers: {
					'Content-Type': 'application/json'
				}
			}
		);
	}
}

/**
 * 主函数
 */
async function main() {
	const { port, noBrowser, help, apiTarget } = parseArgs();

	if (help) {
		showHelp();
		process.exit(0);
	}

	console.log(`
╔════════════════════════════════════════════════╗
║   Cortex Memory Insights v${version.padEnd(18)}    ║
║   Standalone Server                            ║
╚════════════════════════════════════════════════╝
`);

	console.log(`📦 Version: ${version}`);
	console.log(`🔨 Build: ${buildTime}`);
	console.log(`🌐 Starting server...`);

	// 获取dist目录
	const distPath = getDistPath();

	if (!existsSync(distPath)) {
		console.error(`\n❌ Error: dist/ directory not found at ${distPath}`);
		console.error(`\n💡 Please run 'bun run build' first to generate the dist/ directory\n`);
		process.exit(1);
	}

	console.log(`📁 Serving from: ${distPath}`);
	console.log(`🔗 API proxy to: ${apiTarget}`);

	// 启动HTTP服务器
	const server = Bun.serve({
		port,
		hostname: HOST,

		async fetch(req) {
			const url = new URL(req.url);
			let pathname = url.pathname;

			// API代理：/api/v2/* 和 /health
			if (pathname.startsWith('/api/v2') || pathname === '/health') {
				return proxyRequest(req, apiTarget);
			}

			// 根路径返回index.html
			if (pathname === '/') {
				pathname = '/index.html';
			}

			// 构建文件路径
			const filePath = join(distPath, pathname);

			// 检查文件是否存在
			const file = Bun.file(filePath);
			const exists = await file.exists();

			if (exists) {
				return new Response(file, {
					headers: {
						'Content-Type': getMimeType(pathname),
						'Cache-Control': pathname === '/index.html' ? 'no-cache' : 'public, max-age=31536000'
					}
				});
			}

			// 如果文件不存在，返回index.html（支持前端路由）
			// 除非是明确的API路径或静态资源
			if (!pathname.startsWith('/api') && !pathname.includes('.')) {
				const indexFile = Bun.file(join(distPath, 'index.html'));
				return new Response(indexFile, {
					headers: {
						'Content-Type': 'text/html',
						'Cache-Control': 'no-cache'
					}
				});
			}

			return new Response('Not Found', { status: 404 });
		},

		error(error) {
			console.error('❌ Server error:', error);
			return new Response('Internal Server Error', { status: 500 });
		}
	});

	const serverUrl = `http://${HOST}:${port}`;
	console.log(`\n✅ Server running at: ${serverUrl}`);
	console.log(`📁 Serving: Cortex Memory Insights UI`);
	console.log(`🔗 Proxying: /api/v2/* → ${apiTarget}/api/v2/*`);
	console.log(`🔗 Proxying: /health → ${apiTarget}/health`);

	// 自动打开浏览器
	if (!noBrowser) {
		setTimeout(() => {
			openBrowser(serverUrl);
		}, 500); // 延迟500ms确保服务器完全启动
	} else {
		console.log(`📝 Browser auto-open disabled. Please visit ${serverUrl} manually.`);
	}

	console.log(`\n💡 Press Ctrl+C to stop the server\n`);

	// 优雅关闭
	const shutdown = () => {
		console.log('\n👋 Shutting down server...');
		server.stop();
		process.exit(0);
	};

	process.on('SIGINT', shutdown);
	process.on('SIGTERM', shutdown);
}

// 启动服务器
main().catch((error) => {
	console.error('❌ Fatal error:', error);
	process.exit(1);
});
