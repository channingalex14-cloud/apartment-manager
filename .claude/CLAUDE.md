# CLAUDE.md

## 项目信息

- **版本**：2.0.5（更新 2026-04-27）
- **状态**：Phase 4 完成（权限系统）
- **数据库 Schema**：2.1.1
- **包管理**：pnpm

## 技术栈

- **前端**：Vue 3 + TypeScript + Element Plus + ECharts + Pinia + Zod + xlsx
- **后端**：Rust + Tauri 2.0 + rusqlite + tracing + bcrypt
- **测试**：vitest
- **数据库**：SQLite

## 项目结构

```text
src/
├── services/     # API 服务层（16个）
├── stores/       # Pinia 状态管理（6个）
├── views/        # 页面组件（14个）
├── types/        # TypeScript 类型定义
└── utils/        # 工具函数（money.ts, date.ts）

src-tauri/
├── src/          # Rust 后端源码
└── database/     # 数据库相关
```

## 工作流原则

### 文档/统计数据更新

**更新任何文档（README、统计类内容）前，必须执行完整流程：**

1. 实际运行命令获取真实数据（grep / wc / tree 等）
2. 把数字展示给用户确认
3. 用户确认后再编辑
4. 编辑后再次验证文档内容是否与实际吻合

禁止估算数字、引用旧数据，或在未经用户确认的情况下提交统计类更新。

### UI / 样式改动

改动表格、CSS、尺寸（mm/px）前：

1. 描述预期的 HTML / CSS 结构
2. 等用户确认后再编辑
3. 不清楚时主动问，不要猜

### 文件编辑后

- 运行类型检查（tsc / cargo check）确认无误再报告完成
- 涉及 Rust：`cargo check --manifest-path=src-tauri/Cargo.toml`
- 涉及 TypeScript：`vue-tsc --noEmit` 或 `npm run type-check`

### WebFetch / 网络访问

访问外部 URL 受限时，立即告知用户，询问是否手动提供内容，不要反复尝试。

## 语言偏好

- **TypeScript 优先**，除非用户明确要求 Rust 或 Python
- 不写 JavaScript
- 优先 TypeScript 而非 JS

## 工作风格

- 迭代式：先探索再细化，用户核查后提交
- 任务复杂时用 TodoWrite 拆解步骤
- 保持主动自审，不等用户发现错误

## 代码审查（/simplify）

修改代码后，审查改动是否：

### 1. 代码复用

- 是否有重复逻辑可以用现有工具函数替代
- 是否有手写的字符串操作、路径处理可以被现有工具替代
- inline 逻辑是否能用现有工具函数替代

### 2. 代码质量

- 冗余状态：是否有多余的 state、缓存值可以推导
- 参数膨胀：是否在给函数加参数而不是通用化
- 复制粘贴变体：是否有近重复代码块需要统一
- 字符串类型：是否有用 raw string 而不是常量/枚举

### 3. 效率

- 不必要的工作：重复计算、重复文件读取、重复 API 调用
- 错过并发：独立操作是否串行执行可以并行
- 热路径膨胀：启动或 per-request/per-render 热路径是否有阻塞
- 不必要的存在检查：TOCTOU 反模式

## 代码注释

- **禁止添加任何注释**（除非是解释 WHY 的业务逻辑约束）
- 代码本身应自解释，变量/函数名要清晰

## 安全相关

- 密码用 bcrypt 哈希
- 不在代码中硬编码凭证
- 防御性安全：不协助恶意代码
