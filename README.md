# 新逸公寓管理系统 V2.0.5

**版本**：V2.0.5
**更新**：2026-04-27
**技术栈**：Tauri 2.0 + Vue 3 + TypeScript + Rust + SQLite
**状态**：Phase 4 完成（权限系统）

---

## 版本信息

| 组件 | 版本 |
| --- | --- |
| 前端 (package.json) | 2.0.5 |
| 后端 (Cargo.toml) | 2.0.5 |
| 数据库 Schema | 2.1.1 |
| 本文档 | V2.0.5 |

---

## 项目结构

```text
apartment-manager/
├── package.json              # 前端依赖
├── vite.config.ts           # Vite 构建配置
├── tsconfig.json            # TypeScript 配置
├── index.html               # 入口 HTML
│
├── src/                     # 前端源码
│   ├── main.ts              # 前端入口
│   ├── App.vue              # 根组件
│   ├── env.d.ts             # 类型声明
│   ├── router/
│   │   └── index.ts         # 路由配置 + Layout
│   ├── assets/styles/
│   │   ├── _variables.scss  # SCSS 变量
│   │   ├── _overrides.scss  # Element Plus 覆盖
│   │   └── global.scss      # 全局样式
│   ├── components/common/
│   │   ├── AppLayout.vue     # 布局组件（侧边栏+头部）
│   │   ├── ThemeToggle.vue   # 主题切换
│   │   └── CssDiagnostic.vue # CSS 变量诊断
│   ├── services/            # API 服务层（16个）
│   │   ├── api.ts           # invoke 封装 + 错误处理
│   │   ├── room.service.ts
│   │   ├── tenant.service.ts
│   │   ├── lease.service.ts
│   │   ├── bill.service.ts
│   │   ├── payment.service.ts
│   │   ├── deposit.service.ts
│   │   ├── config.service.ts
│   │   ├── meter-reading.service.ts
│   │   ├── report.service.ts
│   │   ├── reminder.service.ts
│   │   ├── document.service.ts
│   │   ├── import.service.ts
│   │   ├── maintenance.service.ts
│   │   └── diagnostic.service.ts
│   ├── stores/              # Pinia 状态管理（6个）
│   │   ├── index.ts          # store 注册
│   │   ├── room.ts
│   │   ├── tenant.ts
│   │   ├── lease.ts
│   │   ├── bill.ts
│   │   ├── system.ts         # 系统配置
│   │   └── ui.ts            # UI 状态（侧边栏折叠/主题）
│   ├── types/               # TypeScript 类型
│   │   ├── room.ts
│   │   ├── tenant.ts
│   │   ├── lease.ts
│   │   ├── bill.ts
│   │   ├── payment.ts
│   │   └── config.ts
│   ├── utils/               # 工具函数
│   │   ├── money.ts        # 金额转换（分↔元）
│   │   └── date.ts         # 日期格式化
│   └── views/               # 页面组件（14个）
│       ├── Dashboard.vue     # 首页（含月度报表摘要）
│       ├── RoomList.vue     # 房态管理（核心页面）
│       ├── TenantList.vue   # 租客列表
│       ├── LeaseList.vue    # 合同列表
│       ├── BillList.vue     # 账单列表
│       ├── BillGenerate.vue # 账单生成
│       ├── BillDetail.vue   # 账单详情
│       ├── PaymentList.vue  # 缴费记录
│       ├── DepositLedger.vue # 押金台账
│       ├── Reports.vue      # 月度报表
│       ├── Reminders.vue    # 通知提醒
│       ├── Documents.vue    # 文档管理
│       └── Settings.vue     # 系统设置
│
└── src-tauri/              # Rust 后端
    ├── Cargo.toml           # Rust 依赖
    ├── tauri.conf.json      # Tauri 配置
    ├── build.rs
    └── src/
        ├── main.rs          # 主入口
        ├── lib.rs           # 库入口
        ├── errors.rs        # 统一错误类型
        ├── utils.rs         # 工具模块
        │   └── money.rs     # 金额处理
        ├── models/          # 数据模型
        │   ├── room.rs       # RoomStatus 枚举
        │   ├── tenant.rs
        │   ├── lease.rs      # LeaseStatus 枚举
        │   ├── bill.rs       # BillStatus 枚举
        │   ├── payment.rs
        │   ├── deposit.rs
        │   ├── config.rs
        │   ├── meter_reading.rs
        │   ├── report.rs
        │   ├── reminder.rs
        │   └── document.rs
        ├── db/              # 数据库层
        │   ├── connection.rs  # 连接管理 + 事务 + VACUUM
        │   ├── migrations.rs  # 迁移管理
        │   ├── queries.rs     # SQL 查询
        │   └── test_helpers.rs # 测试工具
        ├── services/        # 业务逻辑层
        │   ├── lease_service.rs     # 合同状态机
        │   ├── bill_service.rs      # 账单生成 + BillStatus 状态机
        │   ├── payment_service.rs   # 缴费处理
        │   ├── deposit_service.rs   # 押金台账
        │   ├── meter_reading_service.rs # 抄表录入
        │   ├── report_service.rs    # 月度汇总
        │   ├── reminder_service.rs  # 提醒 CRUD
        │   └── document_service.rs # 文档 CRUD
        ├── commands/        # Tauri 命令（58个，14个文件）
        │   ├── room_commands.rs
        │   ├── tenant_commands.rs
        │   ├── lease_commands.rs
        │   ├── bill_commands.rs
        │   ├── payment_commands.rs
        │   ├── deposit_commands.rs
        │   ├── config_commands.rs
        │   ├── meter_reading_commands.rs
        │   ├── report_commands.rs
        │   ├── reminder_commands.rs
        │   ├── document_commands.rs
        │   ├── import_commands.rs
        │   ├── maintenance_commands.rs
        │   └── diagnostic.rs
        └── interceptors/    # 命令拦截器
            └── logging.rs   # 操作日志
```

### 数据库：15 表（不含 schema_version 管理表）

```text
rooms / tenants / leases / monthly_bills / payments /
deposit_ledger / meter_readings / room_status_log /
tenant_history / system_config / notice_templates /
documents / reminders / operation_logs / monthly_summary_cache
```

> 注：另有 `schema_version` 表用于数据库版本管理（migrations.rs 自动维护）。

### Phase 0 完成项

- [x] 前端项目配置（package.json, vite.config.ts, tsconfig.json）
- [x] Rust 后端配置（Cargo.toml, tauri.conf.json）
- [x] 完整源码目录结构
- [x] 数据库初始化脚本（15表 + 107房间）

### Phase 1 完成项

#### 后端实现

- [x] Models 层：Room/Tenant/Lease/Bill/Payment/Deposit/Config
- [x] DB Layer：Connection + 事务 + Migrations + Queries
- [x] Services 层：

  - [x] LeaseService：check_in / check_out / mark_violation
  - [x] BillService：generate_monthly_bills
  - [x] PaymentService：record_payment
  - [x] DepositService：get_deposit_ledger / receive_deposit / refund_deposit

- [x] Commands 层：7个命令模块
- [x] Interceptors：操作日志

#### 前端

- [x] Pinia Stores：room / tenant / lease / bill
- [x] Services：7个 API 服务
- [x] Types：完整的 TypeScript 类型定义
- [x] Layout 组件：侧边栏 + 头部导航
- [x] Dashboard 页面：统计卡片 + 入住率 + 待缴费列表
- [x] RoomList 页面：筛选 + 搜索 + 表格展示

### Phase 2 完成项（M1-M4）

#### 核心变更：抄表录入与账单生成解耦

```text
旧数据流：generate_room_bill → 读 rooms.meter_current → INSERT monthly_bills
新数据流：抄表录入 → INSERT meter_readings → generate_room_bill → 读 meter_readings
```

#### Rust 后端

- [x] `meter_readings` 表 + 迁移
- [x] `MeterReadingService`：单条/批量录入/倒拨校验/换表标记
- [x] `BillStatus` 状态机：Pending → Partial/Paid/Voided
- [x] 账单操作 TOCTOU 防护：confirm_bill_paid / partial_pay_bill / void_bill
- [x] 作废账单重新生成（UPDATE 恢复，非 INSERT）
- [x] 账单详情查询（含 JOIN 水电读数）

#### 前端

- [x] BillList.vue：账单列表页
- [x] BillDetail.vue：账单详情页（操作按钮按状态显示/隐藏）
- [x] meter-reading.service.ts：抄表服务

#### 测试覆盖

- [x] 89 个集成测试全绿
- [x] TOCTOU 回归测试：6 个
- [x] MeterReadingService 测试：8 个
- [x] BillService 核心测试：35 个
- [x] BillAction 测试：9 个

### Phase 3 完成项

- [x] ReportService + report_commands.rs：月度汇总缓存
- [x] ReminderService + reminder_commands.rs：提醒 CRUD
- [x] DocumentService + document_commands.rs：文档管理
- [x] Reports.vue：月度报表页
- [x] Reminders.vue：通知提醒页
- [x] Documents.vue：文档管理页
- [x] vacuum_database：数据库压缩
- [x] ThemeToggle + 暗色主题支持
- [x] CssDiagnostic：CSS 变量诊断组件
- [x] 导入命令：import_commands（Excel 导入 + 状态归一化）

### 架构原则

1. **前端禁止直接操作数据库** - 所有 DB 访问走 `invoke()` → Rust Commands
2. **金额统一用分** - 前端转换函数在 `utils/money.ts`
3. **状态机在 Rust Service 层** - `LeaseStatus` / `BillStatus` 枚举
4. **事务统一封装** - `db/connection.rs` 的 `transaction()` 方法
5. **耗时操作用 spawn_blocking** - `bill_commands.rs` 中使用
6. **抄表录入与账单生成解耦** - `meter_readings` 表作为中间层
7. **TOCTOU 防护** - 所有写前读取在事务内

---

## 状态机

### LeaseStatus 流转

```text
草稿 → 生效中 ⟷ 违约中 → 待结算 → 已退房 → 已归档
  ↓              ↑
  已作废         └── 违约恢复（Violation → Active）
```

### BillStatus 流转

```text
待缴费 ──[确认支付]──→ 已支付（终态）
  ├──[部分支付]──→ 部分支付 ──[确认支付]──→ 已支付（终态）
  │                └──[作废]──→ 已作废（终态）
  └──[作废]──→ 已作废（终态）──[重新生成]──→ 待缴费（UPDATE 恢复）
```

### RoomStatus（8 种状态）

| 状态 | 中文 | 允许入住 | 允许退房 | 显示合同 |
| ---- | ---- | -------- | -------- | -------- |
| 空房 | 空房 | ✓ | ✗ | ✗ |
| 在租 | 在租 | ✗ | ✓ | ✓ |
| 新租 | 新租 | ✗ | ✓ | ✓ |
| 员工 | 员工 | ✗ | ✗ | ✗ |
| 管理 | 管理 | ✗ | ✗ | ✗ |
| 违约 | 违约 | ✗ | ✓ | ✗ |
| 待清洁 | 待清洁 | ✓ | ✗ | ✗ |
| 维修中 | 维修中 | ✗ | ✗ | ✗ |

**不显示合同信息的 6 种**：空房 / 员工 / 管理 / 违约 / 待清洁 / 维修中

### 状态手动切换规则（重要）

在房间详情抽屉中手动修改状态时，**只能**在以下特殊状态之间切换：

| 可自由切换的状态组合 |
| ------------------- |
| 空房 ↔ 维修中 / 管理 / 员工 |
| 维修中 ↔ 空房 / 管理 / 员工 |
| 管理 ↔ 空房 / 维修中 / 员工 |
| 员工 ↔ 空房 / 维修中 / 管理 |
| 待清洁 ↔ 空房 / 维修中 / 管理 / 员工 |

**以下状态不允许手动直接修改**（必须走状态机流程）：

| 当前状态 | 目标状态 | 正确操作 |
| -------- | -------- | -------- |
| 在租 / 新租 / 违约 | 空房 | 必须执行**退房**（check_out） |
| 在租 / 新租 / 违约 | 违约 | 必须执行**违约标记**（mark_violation） |
| 违约 | 在租 / 新租 | 必须执行**违约恢复**（recover_from_violation） |

违反上述规则会报错：

```text
房间状态 '新租' 不允许直接修改为 '空房'。在租/新租/违约状态必须通过退房或违约处理流程变更。
```

**判断逻辑**：`src-tauri/src/models/room.rs` → `RoomStatus::allows_manual_transition_to()`

---

## 升级注意事项

### 数据存储位置

用户数据独立存储在系统应用数据目录，与 exe 安装位置无关：

| 操作系统 | 数据库路径 |
|---------|-----------|
| Windows | `%LOCALAPPDATA%\com.xinyi.apartment-manager\apartment.db` |
| macOS | `~/Library/Application Support/com.xinyi.apartment-manager/` |
| Linux | `~/.local/share/com.xinyi.apartment-manager/` |

### 版本说明

本项目使用两个独立版本号：

| 版本类型 | 当前值 | 说明 |
|---------|-------|------|
| **应用版本** | 2.0.5 | 代码版本，用于发布管理 |
| **数据库 Schema 版本** | 2.1.1 | 数据库结构版本，用于迁移管理 |

两者独立递增，互不影响。

### 升级流程

升级时只需替换 exe 文件，数据不会丢失。启动时会自动检测数据库版本并执行迁移：

```
每次启动流程：
1. 读取 schema_version 表中的当前版本
2. 如果当前版本 < 目标版本 (2.1.1)
3. 自动执行增量迁移 (2.0.4 → 2.0.5 → ... → 2.1.1)
4. 更新 schema_version 表记录
```

### 迁移规则

- **禁止直接修改** `INIT_SQL`（初始化脚本）
- 新增表或字段应在 `run_incremental_migrations()` 中添加增量迁移
- 迁移函数命名格式：`if version_less_than(from_version, "X.Y.Z")`

---

## 启动方式

```bash

# 安装依赖
pnpm install

# 下载 Rust 依赖
cd src-tauri && cargo fetch && cd ..

# 启动开发服务器
pnpm tauri dev
```

---

## Phase 4 完成项（权限系统）

- [x] 操作员权限管理（后端 + 前端登录页面）
- [x] 路由守卫（未登录重定向到登录页）
- [x] 用户会话管理（Token + localStorage 持久化）
- [x] 登出功能

### Phase 4 待完善

- [ ] require_permission! 宏应用到更多命令
- [ ] 数据导入导出（Excel）
- [ ] 批量操作优化
- [ ] 通知提醒（微信/短信）
- [ ] 移动端适配

---

## 相关文档

| 文档 | 说明 |
| ---- | ---- |
| [reports/V2.0.5 开发指南.md](reports/V2.0.5 开发指南.md) | 开发规范与状态机文档 |
| [reports/V2.0.5 数据库完整脚本.md](reports/V2.0.5 数据库完整脚本.md) | 完整数据库 Schema |
| [reports/V2.0.5 缺陷扫描报告.md](reports/V2.0.5 缺陷扫描报告.md) | 缺陷修复与测试覆盖报告 |
| [reports/V2.0.5 项目架构概览.md](reports/V2.0.5 项目架构概览.md) | 系统架构与模块概览 |
| [reports/V2.0.5 数据库设计详解.md](reports/V2.0.5 数据库设计详解.md) | 数据库表结构详解 |
| [reports/V2.0.5 前端开发指南.md](reports/V2.0.5 前端开发指南.md) | Vue3 + TS 开发规范 |
| [reports/V2.0.5 房态UI设计决策.md](reports/V2.0.5 房态UI设计决策.md) | 房态 UI 配色与交互 |
| [DEVELOPMENT_PLAN.md](DEVELOPMENT_PLAN.md) | 开发计划 |
