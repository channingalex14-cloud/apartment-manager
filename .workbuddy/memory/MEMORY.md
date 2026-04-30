# MEMORY.md

## 项目概述

- **项目名称**：apartment-manager（公寓管理系统）
- **技术栈**：Tauri 2.0 + Vue 3 + Element Plus + SQLite
- **当前版本**：V2.0.5 → V3.0.0 迁移中
- **开发阶段**：V3.0.0 架构重构规划

## V3.0.0 升级计划（2026-04-28）

### 技术决策

**数据库层升级**：
- 从 rusqlite（同步）迁移到 tauri-plugin-sql（异步）
- 底层使用 sqlx（官方推荐）
- 支持未来切换到 PostgreSQL

**迁移策略**：
- 方案 B：并行运行 + 分阶段迁移
- 预计工期：4-5 周
- 风险等级：中等（可控）

**关键发现**：
- SQLite 不支持 DROP COLUMN（DDL 限制）
- tauri-plugin-sql 是官方最佳实践
- pgloader 是最佳迁移工具

### 迁移计划

| 阶段 | 内容 | 时间 |
|-----|------|------|
| Phase 0 | 环境准备 | 1 天 |
| Phase 1 | 异步架构升级 | 1 周 |
| Phase 2 | Repository 抽象层 | 1 周 |
| Phase 3 | 迁移系统模块化 | 1 周 |
| Phase 4 | 清理和发布 | 0.5 天 |

**总工期**：约 3.5 周（2026-04-28 更新）

**关键技术决策**（专家评审融入）：
- **技术选型**：直接使用 sqlx，不使用 tauri-plugin-sql（2026-04-28 专家确认）
- **Executor 泛型模式**：Repository 方法使用 `Executor<'e, Database = Sqlite>`，支持 Pool 和 Transaction
- **RepositoryError 设计**：使用 `Database(String)` 包装错误，不泄露 sqlx 依赖
- **Mock 异步锁**：使用 `tokio::sync::Mutex`，所有 `.lock()` 改为 `.lock().await`
- **迁移自举**：`ensure_migration_table()` 在 `run()` 入口调用，解决表不存在问题
- **迁移事务包裹**：每个迁移在独立事务中执行，失败回滚
- **MigrationError 独立**：不依赖 AppError，单独定义错误类型
- 编译时 Feature Flag：`legacy-db` / `new-db` 隔离新旧代码
- 连接池配置：优先测试 `max_connections: 1`（SQLite 单连接 + WAL）

### 技术栈升级

| 组件 | V2.0.5 | V3.0.0 |
|-----|--------|--------|
| 数据库库 | rusqlite（同步） | tauri-plugin-sql（异步） |
| 底层库 | rusqlite | sqlx |
| 连接池 | 无 | sqlx 内置连接池 |
| 类型安全 | 手动转换 | 编译时检查 |

## 技术架构

### 前端
- Vue 3 + TypeScript
- Element Plus UI 框架
- Pinia 状态管理
- Vue Router 路由

### 后端
- Rust + Tauri 2.0
- tauri-plugin-sql（数据访问）
- 异步架构（tokio）

### 数据库
- SQLite（当前）/ PostgreSQL（未来可选）
- 主要表：rooms, tenants, leases, monthly_bills, payments, deposit_ledger

## 开发规范

### Git 分支策略
- main：生产稳定版本
- develop：开发分支
- feature/*：功能分支
- v3.0.0-postgres-migration：迁移分支

### 代码规范
- Rust：遵循官方 Rust 风格指南
- TypeScript：ESLint + Prettier
- 提交信息：Conventional Commits

### 测试策略
- 单元测试：关键业务逻辑
- 集成测试：API 端点
- E2E 测试：主要用户流程
- 迁移验证：数据一致性检查
