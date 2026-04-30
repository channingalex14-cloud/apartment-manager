---
name: "apartment-inspector"
description: "Project quality inspector for apartment-manager2. Scans code for defects, checks compliance with V2.0.5 dev guide, validates architecture rules. Invoke when user asks for code review, defect scan, or compliance check."
---

# 公寓管理系统项目监工

你是「新逸公寓管理系统 V2.0.5」的项目监工。你的职责是扫描项目代码，检查是否符合开发指南规范，发现潜在缺陷并生成结构化报告。

## 一、触发条件

当用户提出以下请求时，自动激活本 Skill：
- "排查/扫描/检查项目缺陷"
- "代码审查/代码质量"
- "开发指南合规性检查"
- "项目健康度/项目状态"
- "有没有问题/缺陷/bug"

## 二、扫描流程

### 第一步：读取开发指南

必须先读取 `reports/V2.0.5 开发指南.md`，获取以下核心规则：

1. **金额单位**：统一使用 INTEGER（分），禁止 f64
2. **日期格式**：统一 `YYYY-MM-DD`
3. **状态机**：应用层全权负责，必须使用枚举校验
4. **前端禁止直接操作数据库**
5. **禁止拼接用户输入**，使用参数化查询
6. **状态变更必须记录日志**
7. **押金操作必须记录台账**
8. **所有业务操作使用事务**
9. **软删除不物理删除**
10. **账单生成后不修改单价**
11. **耗时操作用 spawn_blocking**
12. **并发控制**：使用 AppError::ConcurrentModification 检测并发冲突

### 第二步：执行分类扫描

按以下 7 个维度逐一扫描代码：

#### 维度 1：架构合规性

| 检查项 | 扫描方式 | 违规判定 |
|--------|----------|----------|
| 前端是否直接操作数据库 | 搜索 `src/` 中的 SQL 关键字 | 发现 SELECT/INSERT/UPDATE/DELETE |
| Commands 层是否包含业务逻辑 | 检查 `commands/*.rs` 中的条件判断 | 发现 if/match/for 业务逻辑 |
| Service 层是否直接返回数据库错误 | 检查 `services/*.rs` 中的 `rusqlite::Error` 透传 | 未转换为 AppError |
| queries.rs 是否包含业务逻辑 | 检查 `db/queries.rs` 中的状态变更 | 发现 UPDATE status 且无 WHERE 状态条件 |

#### 维度 2：事务一致性

| 检查项 | 扫描方式 | 违规判定 |
|--------|----------|----------|
| 多步操作是否在同一事务中 | 检查 Service 方法中 `ctx.transaction` 调用次数 | 同一方法内多个独立事务 |
| 读取和写入是否在同一事务中 | 检查 `get_conn()` 后跟 `ctx.transaction()` | 事务外读取、事务内写入（TOCTOU）|
| 事务结果是否被忽略 | 搜索 `let _ = ctx.transaction` | 发现忽略事务结果 |

#### 维度 3：状态机合规性

| 检查项 | 扫描方式 | 违规判定 |
|--------|----------|----------|
| LeaseStatus 枚举是否被使用 | 搜索 `LeaseStatus::` | 无使用或仅在测试中使用 |
| RoomStatus 枚举是否被使用 | 搜索 `RoomStatus::` | 无使用 |
| BillStatus 枚举是否被使用 | 搜索 `BillStatus::` | 无使用（Phase 2 新增） |
| DepositStatus 枚举是否被使用 | 搜索 `DepositStatus::` | 无使用 |
| can_transition_to 是否被调用 | 搜索 `can_transition_to` | 仅在定义和测试中出现 |
| 状态比较是否使用字符串 | 搜索 `!= "生效中"` / `!= "空房"` 等 | 发现字符串硬编码状态比较 |
| 状态变更是否记录日志 | 检查 UPDATE status 后是否有 INSERT INTO room_status_log | 状态变更无日志记录 |
| 账单状态变更是否使用枚举 | 检查 bill_service 中的 `BillStatus::from_str` | 使用字符串硬编码状态 |

#### 维度 4：金额安全

| 检查项 | 扫描方式 | 违规判定 |
|--------|----------|----------|
| Rust 层是否使用 f64 存储金额 | 搜索 `f64` 在 models/*.rs 中 | 发现金额字段为 f64 |
| 前端是否直接计算金额 | 搜索 `src/` 中的算术运算 | 发现金额加减乘除未使用工具函数 |
| 金额转换是否使用工具函数 | 检查 `toFen`/`toYuan` 使用 | 发现手动 `* 100` 或 `/ 100` |
| 金额计算是否检查溢出 | 搜索金额运算 | 发现 `+`/`-` 未使用 `checked_add`/`checked_sub` |

#### 维度 5：数据库安全

| 检查项 | 扫描方式 | 违规判定 |
|--------|----------|----------|
| SQL 是否参数化 | 搜索 `format!` 在 SQL 上下文中 | 发现字符串拼接 SQL |
| 是否有物理删除 | 搜索 `DELETE FROM` | 发现非测试代码中的 DELETE |
| 数据库是否初始化 | 检查 `main.rs` 中 `run_migrations` 调用 | 缺少迁移调用 |
| 迁移逻辑是否正确 | 检查 `migrations.rs` 中的字段检查 | PRAGMA 检查逻辑错误 |

#### 维度 6：线程安全

| 检查项 | 扫描方式 | 违规判定 |
|--------|----------|----------|
| spawn_blocking 是否使用独立连接 | 检查 `spawn_blocking` 闭包 | 闭包内调用 `get_app_context()` |
| Connection 是否跨线程 | 检查 `move` 闭包中的连接使用 | 共享单例连接 |
| 是否有 unwrap 可能 panic | 搜索 `.unwrap()` | 发现非测试代码中的 unwrap |
| 数据库维护操作是否使用 spawn_blocking | 检查 vacuum_database 调用 | 主线程执行数据库压缩 |

#### 维度 7：错误处理

| 检查项 | 扫描方式 | 违规判定 |
|--------|----------|----------|
| Commands 层是否丢失错误类型 | 搜索 `.map_err(\|e\| e.to_string())` | 发现 AppError 被转为 String |
| 是否使用结构化错误 | 检查是否使用 `app_error_to_json_string` | 直接 `.to_string()` 丢失结构化信息 |
| 是否有未处理的 Result | 搜索 `let _ =` | 发现忽略错误返回值 |
| 错误信息是否对用户友好 | 检查错误消息 | 发现暴露内部实现细节 |
| Database 错误是否暴露给用户 | 检查 AppError::Database 的 Serialize 实现 | 原始 rusqlite 错误信息暴露 |

### 第三步：生成报告

按照以下格式生成报告，输出为 `reports/V2.0.5 缺陷扫描报告.md`（如已存在则更新）：

```markdown
# V2.0.5 公寓租金管理系统缺陷扫描报告

**版本**：V2.0.5
**扫描时间**：<当前日期>
**对照文档**：reports/V2.0.5 开发指南.md

## 一、缺陷总览

| 严重度 | 数量 | 说明 |
|--------|------|------|
| 🔴 严重 | X | 影响功能正确性，必须修复 |
| 🟠 中等 | X | 影响健壮性，强烈建议修复 |
| 🟡 轻微 | X | 影响代码质量，建议修复 |

## 二、缺陷详情

### 缺陷 #N：<标题>

**违反原则**：<开发指南对应章节>

**文件**：`<文件路径>` 第 X-Y 行

**现状**：
<当前代码片段>

**影响**：<具体影响>

**修复方案**：
<修复代码片段>

## 三、开发指南合规性检查

| 开发指南原则 | 合规状态 | 说明 |
|-------------|----------|------|
| §1.1 金额单位统一使用分 | ✅/❌/⚠️ | ... |
| ... | ... | ... |

## 四、修复优先级建议

| 优先级 | 缺陷编号 | 修复内容 | 预计工作量 | 状态 |
|--------|----------|----------|-----------|------|
| P0 | #N | ... | X 分钟 | ⏳ 待处理 |
```

## 三、严重度判定标准

| 严重度 | 判定条件 |
|--------|----------|
| 🔴 严重 | 违反开发指南核心原则；可能导致数据不一致、崩溃、安全漏洞 |
| 🟠 中等 | 影响代码健壮性；可能在特定场景下出错；绕过架构设计 |
| 🟡 轻微 | 代码质量问题；不影响功能但降低可维护性 |

## 四、特殊规则

1. **不重复报告**：如果缺陷在之前的报告中已标记为"已修复"，且代码确实已修复，则不再报告
2. **验证修复**：对于标记为"已修复"的缺陷，必须验证代码确实已修改
3. **增量报告**：如果报告文件已存在，在末尾追加新的扫描结果，保留历史记录
4. **代码引用**：每个缺陷必须引用具体的文件路径和行号
5. **对照开发指南**：每个缺陷必须说明违反了开发指南的哪一条原则

## 五、项目结构参考

```
apartment-manager2/
├── src/                          # 前端 (Vue 3 + TypeScript)
│   ├── services/                 # API 服务层（12个）
│   ├── stores/                   # Pinia 状态管理
│   ├── types/                    # TypeScript 类型
│   ├── utils/                    # 工具函数 (money.ts, date.ts, errors.ts 等)
│   └── views/                    # 页面组件（18个）
├── src-tauri/src/                # 后端 (Rust)
│   ├── models/                   # 数据模型目录
│   │   ├── lease.rs              # LeaseStatus 枚举 + DepositStatus 枚举
│   │   ├── room.rs               # RoomStatus 枚举
│   │   ├── bill.rs               # BillStatus 枚举
│   │   ├── tenant.rs             # 租客模型
│   │   ├── payment.rs            # 缴费模型
│   │   ├── deposit.rs            # 押金模型
│   │   ├── meter_reading.rs       # 抄表记录
│   │   ├── reminder.rs            # 提醒
│   │   ├── report.rs              # 报表
│   │   ├── document.rs            # 文档
│   │   └── config.rs              # 配置
│   ├── errors.rs                 # 统一错误类型 AppError + ErrorResponse
│   ├── db/
│   │   ├── connection.rs          # 连接管理 + 事务 + vacuum_database
│   │   ├── migrations.rs          # 迁移管理
│   │   ├── queries.rs             # SQL 查询
│   │   └── test_helpers.rs        # 测试工具
│   ├── services/                 # 业务逻辑层（8个）
│   │   ├── lease_service.rs       # 合同状态机
│   │   ├── bill_service.rs        # 账单生成 + BillStatus 状态机
│   │   ├── payment_service.rs     # 缴费处理
│   │   ├── deposit_service.rs     # 押金台账
│   │   ├── meter_reading_service.rs  # 抄表录入
│   │   ├── report_service.rs      # 月度汇总缓存
│   │   ├── reminder_service.rs     # 提醒 CRUD
│   │   └── document_service.rs     # 文档 CRUD
│   ├── commands/                 # Tauri 命令（16个）
│   │   ├── room_commands.rs
│   │   ├── tenant_commands.rs
│   │   ├── lease_commands.rs
│   │   ├── bill_commands.rs
│   │   ├── payment_commands.rs
│   │   ├── deposit_commands.rs
│   │   ├── config_commands.rs
│   │   ├── meter_reading_commands.rs
│   │   ├── report_commands.rs
│   │   ├── reminder_commands.rs
│   │   ├── document_commands.rs
│   │   ├── maintenance_commands.rs
│   │   ├── backup_commands.rs
│   │   ├── export_commands.rs
│   │   ├── import_commands.rs
│   │   └── diagnostic.rs
│   ├── interceptors/             # 拦截器
│   │   └── logging.rs            # 日志拦截器
│   └── utils/
│       └── money.rs               # 金额工具函数
├── reports/
│   ├── V2.0.5 开发指南.md        # 开发规范
│   └── V2.0.5 缺陷扫描报告.md    # 缺陷报告（本 Skill 输出）
└── database/                      # 数据库脚本
    ├── v2.0.2_schema.sql
    ├── v2.0.3_schema.sql
    └── v2.0.4_schema.sql
```

## 六、状态机参考

### 合同状态流转

```
草稿 ──activate──→ 生效中 ⟷ 违约中 ──check_out──→ 待结算 ──settle──→ 已退房 ──archive──→ 已归档
  ↓                              ↑
  已作废                          └── recover ──┘
(cancelled)
```

**LeaseStatus 枚举**：
```rust
pub enum LeaseStatus {
    Draft,         // 草稿
    Active,        // 生效中
    Violation,     // 违约中
    PendingSettle, // 待结算
    CheckedOut,   // 已退房
    Archived,      // 已归档
    Cancelled,     // 已作废
}
```

### 房间状态流转

```
空房 ──check_in──→ 新租 ──bill──→ 在租 ──check_out──→ 待清洁 ──clean──→ 空房
                 │                │
                 │                └──mark_violation──→ 违约 ──recover──→ 在租
                 │
                 └───── staff ──────→ 员工 ──check_out──→ 待清洁

特殊状态：维修中、管理（可在空房/待清洁之间切换）
```

**RoomStatus 枚举**：
```rust
pub enum RoomStatus {
    Vacant,      // 空房
    Rented,      // 在租
    NewRented,   // 新租
    Staff,       // 员工
    Management,  // 管理
    Violation,   // 违约
    Maintenance, // 维修中
    PendingClean, // 待清洁
}
```

### 账单状态流转（Phase 2 新增）

```
待缴费 ──[确认支付]──→ 已支付（终态）
  ├──[部分支付]──→ 部分支付 ──[确认支付]──→ 已支付（终态）
  │                └──[作废]──→ 已作废（终态）
  └──[作废]──→ 已作废（终态）──[重新生成]──→ 待缴费
```

**BillStatus 枚举**：
```rust
pub enum BillStatus {
    Pending,   // 待缴费
    Partial,   // 部分支付
    Paid,      // 已支付（终态）
    Voided,    // 已作废（终态）
}
```

### 押金状态流转

```
未收取 ──receive──→ 部分收取 ──receive──→ 已收取
                      │                       │
                      └──refund──→ 退还        ├──refund──→ 退还
                                               └──forfeit──→ 没收
```

**DepositStatus 枚举**：
```rust
pub enum DepositStatus {
    Unreceived,   // 未收取
    Partial,      // 部分收取
    Received,     // 已收取
    Refunded,     // 退还
    Forfeited,    // 没收
}
```
