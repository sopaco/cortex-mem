#!/usr/bin/env bun

/**
 * 构建 cortex-mem-insights 独立可执行文件
 * 
 * 使用方法:
 * bun run build-executable.js
 * 
 * 输出:
 * - dist/cortex-mem-insights (macOS/Linux)
 * - dist/cortex-mem-insights.exe (Windows)
 */

import { $ } from "bun";
import path from "path";

const __dirname = import.meta.dir;
const distDir = path.join(__dirname, "dist");

console.log("🚀 开始构建 cortex-mem-insights 独立可执行文件...\n");

// 1. 先构建前端静态文件
console.log("📦 步骤 1/3: 构建前端静态文件...");
await $`bun run build`;

// 2. 创建入口文件用于编译
console.log("\n📝 步骤 2/3: 准备编译入口...");

// 确保 dist 目录存在
await $`mkdir -p ${distDir}`;

// 3. 使用 Bun 编译成独立可执行文件
console.log("\n🔨 步骤 3/3: 编译独立可执行文件...");

const platform = process.platform;
const arch = process.arch;

// 根据平台选择目标
let target;
if (platform === "darwin") {
  target = arch === "arm64" ? "bun-darwin-arm64" : "bun-darwin-x64";
} else if (platform === "linux") {
  target = arch === "arm64" ? "bun-linux-arm64" : "bun-linux-x64";
} else if (platform === "win32") {
  target = "bun-windows-x64";
}

const outfile = path.join(
  distDir,
  platform === "win32" ? "cortex-mem-insights.exe" : "cortex-mem-insights"
);

// 编译可执行文件
await Bun.build({
  entrypoints: ["./start-prod.js"],
  compile: {
    target,
    outfile,
    // 自动加载 .env 文件
    autoloadDotenv: true,
    autoloadBunfig: false,
    autoloadTsconfig: false,
    autoloadPackageJson: false,
  },
  minify: true,
  sourcemap: "linked",
  bytecode: true, // 启用字节码编译以加快启动速度
});

console.log(`\n✅ 构建完成！`);
console.log(`📁 可执行文件位置: ${outfile}`);
console.log(`📊 文件大小: ${(await Bun.file(outfile).size / 1024 / 1024).toFixed(2)} MB`);

console.log("\n🎯 使用方法:");
console.log(`   ${outfile}`);
console.log("\n💡 提示:");
console.log("   - 可执行文件已包含 Bun 运行时和所有依赖");
console.log("   - 可以直接在目标系统上运行,无需安装 Node.js 或 Bun");
console.log("   - 确保 cortex-mem-service 服务正在运行");
console.log("   - 可以通过 .env 文件配置环境变量");
