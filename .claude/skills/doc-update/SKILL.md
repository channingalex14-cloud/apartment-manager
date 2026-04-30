---
name: doc-update
description: 柳舟的文档更新工作流。更新 README、C hangelog、版本说明等统计类文档前，必须实测数据，不估算。触发词：更新文档、更新README、更新统计、更新版本号。
---

# doc-update Skill

## 柳舟的文档更新流程

当用户要求更新任何统计类文档时，严格执行以下四步：

### 第一步：实测数据
不引用旧数据，不估算数字。
- 统计命令数、版本号、表数量等时，实际运行命令获取数字

### 第二步：展示给用户确认
把数字列表展示出来，等用户确认后再更新，不跳过确认

### 第三步：只更新用户确认的部分
只更新用户同意的部分，不多改

### 第四步：更新后重新验证
更新后再次读取文件，确认内容正确

### 柳舟的项目背景
- 项目路径：H:\vibe coding\Apartment\A1\apartment-manager
- 技术栈：Tauri + Vue3 + TypeScript + Rust
- 当前版本：2.0.4
- GitHub: 有，但还未开始 commit，代码仍在探索阶段

### 常用命令
- 前端类型检查：npm run type-check
- Rust 类型检查：cargo check
- 前端构建：npm run build
- Rust 构建：cargo build
