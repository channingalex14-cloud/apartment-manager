---
name: tauri-debug
description: Tauri 开发常见问题排查流程。触发词：窗口闪烁、端口冲突、Tauri 调试、Rust 编译错误、热重载问题、命令不响应。
---

# Tauri 调试技能

## 窗口闪烁 / 白屏 / 崩溃

### 排查步骤

1. **检查端口占用**
   ```powershell
   netstat -ano | findstr 1420
   netstat -ano | findstr 9222
   ```
   如有占用，终止进程：
   ```powershell
   taskkill /F /PID <PID>
   ```

2. **检查 src-tauri 内是否有 logs/ 或其他频繁写入的目录**
   - **根因**：logs 目录在 src-tauri 内会触发文件监视器循环，导致无限重载
   - **解决**：将 logs 目录移到 src-tauri 外部，或在 tauri.conf.json 中排除

3. **检查文件监视器循环**
   - 查看 `tauri.conf.json` 的 `build.dev.beforeDevCommand` 和 `beforeBuildCommand`
   - 确认没有监视 src-tauri 内部目录

4. **清理重建**
   ```bash
   cd src-tauri
   cargo clean
   cargo build
   cd ..
   pnpm install
   pnpm tauri dev
   ```

---

## Rust 编译错误

### 常见问题

1. **.d 文件损坏**
   ```bash
   cd src-tauri
   cargo clean
   cargo build
   ```

2. **路径中有中文或特殊字符**
   - 检查项目路径是否包含中文、空格、特殊字符
   - Rust 编译器对路径编码敏感

3. **依赖版本冲突**
   ```bash
   cargo update
   cargo build
   ```

4. **target 目录过大**
   ```bash
   cargo clean
   # 然后重新构建
   ```

---

## 前端热重载问题

### 排查步骤

1. **检查 vite.config.ts 端口配置**
   ```typescript
   export default defineConfig({
     server: {
       port: 1420,
       strictPort: true,
     },
   })
   ```

2. **检查 tauri.conf.json 权限配置**
   - 确认 `src-tauri/capabilities/` 中的权限配置正确
   - 检查 `devUrl` 是否与 vite 端口一致

3. **检查环境变量**
   - 确认 `.env` 文件中的端口配置
   - 检查 `TAURI_DEV_HOST` 环境变量

---

## Tauri 命令不响应

### 排查步骤

1. **检查命令是否正确注册**
   - 确认 `src-tauri/src/lib.rs` 中有 `invoke_handler` 注册
   - 检查命令函数是否为 `pub async fn` 或 `pub fn`

2. **检查权限配置**
   - 查看 `src-tauri/capabilities/default.json`
   - 确认命令在 `permissions` 中声明

3. **运行 cargo check**
   ```bash
   cd src-tauri
   cargo check
   ```

4. **查看日志**
   - 检查 `target/debug/debug.log`（如果存在）
   - 或在代码中添加 `println!` 调试

---

## WebView2 问题（Windows）

### 常见问题

1. **WebView2 未安装**
   - 下载安装：https://developer.microsoft.com/en-us/microsoft-edge/webview2/

2. **WebView2 版本过旧**
   - 更新到最新版本

3. **WebView2 进程残留**
   ```powershell
   taskkill /F /IM msedgewebview2.exe
   ```

---

## 常用调试命令

```bash
# 前端类型检查
pnpm typecheck

# Rust 类型检查
cd src-tauri && cargo check

# 前端构建
pnpm build

# Rust 构建
cd src-tauri && cargo build

# 完整开发环境启动
pnpm tauri dev

# 清理所有缓存
cd src-tauri && cargo clean && cd ..
rm -rf node_modules
pnpm install
```

---

## 历史问题记录

### 2026-04 窗口闪烁问题

**现象**：开发模式下窗口持续闪烁

**根因**：
1. 端口 1420 被占用
2. src-tauri/logs 目录触发文件监视器循环

**解决**：
1. 终止占用端口的进程
2. 将 logs 目录移出 src-tauri

**教训**：src-tauri 目录内不应放置频繁写入的文件（日志、缓存等）
