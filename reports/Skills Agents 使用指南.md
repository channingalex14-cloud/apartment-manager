# Skills & Agents 使用指南

> 本项目已配置的高级 Skills 和 Agents 汇总
> 更新日期：2026-04-27

---

## 目录

- [Skills（技能）](#skills技能)
  - [Karpathy Guidelines](#1-karpathy-guidelines)
  - [Superpowers 系列](#2-superpowers-系列)
- [Agents（代理）](#agents代理)

---

## Skills（技能）

### 1. Karpathy Guidelines

**文件位置：** `~/.claude/plugins/marketplaces/andrej-karpathy-skills/`

**描述：** 基于 Andrej Karpathy 的 LLM 编码最佳实践指南，减少常见 LLM 编码错误

**触发方式：** 自动激活，或手动调用

```markdown
Use the karpathy-guidelines skill when writing code
```

**核心原则：**

| 原则 | 说明 |
|------|------|
| Think Before Coding | 不要假设，不要隐藏困惑，明确tradeoffs |
| Simplicity First | 最少代码解决问题，不做投机性功能 |
| Surgical Changes | 精确改动，不做大幅重构 |
| Goal-Driven | 定义可验证的成功标准 |

---

### 2. Superpowers 系列

**文件位置：** `~/.claude/plugins/marketplaces/superpowers/skills/`

#### 2.1 systematic-debugging

**描述：** 系统化调试方法论，4阶段根因分析

**触发方式：**

```markdown
/systematic-debugging
```

**描述：** Use when encountering any bug, test failure, or unexpected behavior, before proposing fixes

**核心规则：**
```
NO FIXES WITHOUT ROOT CAUSE INVESTIGATION FIRST
```

**4阶段流程：**
1. Root Cause Investigation（根因调查）
2. Defense in Depth（纵深防御）
3. Condition-based Waiting（条件等待）
4. Verification（验证）

---

#### 2.2 test-driven-development

**描述：** 红-绿-重构 TDD 循环

**触发方式：**

```markdown
/test-driven-development
```

**描述：** Use when implementing any feature or bugfix, before writing implementation code

**核心规则：**
```
NO PRODUCTION CODE WITHOUT A FAILING TEST FIRST
```

**循环：**
```
RED → 写失败测试 → GREEN → 写最小代码通过 → REFACTOR → 重构
```

---

#### 2.3 verification-before-completion

**描述：** 修复完成后验证，确保问题真正解决

**触发方式：**

```markdown
/verification-before-completion
```

**描述：** Use when about to claim work is complete, fixed, or passing, before committing or creating PRs

**核心规则：**
```
NO COMPLETION CLAIMS WITHOUT FRESH VERIFICATION EVIDENCE
```

---

#### 2.4 writing-plans

**描述：** 编写详细实现计划

**触发方式：**

```markdown
/writing-plans
```

**描述：** Use when you have a spec or requirements for a multi-step task, before touching code

**计划保存位置：** `docs/superpowers/plans/YYYY-MM-DD-<feature-name>.md`

---

#### 2.5 brainstorming

**描述：** 头脑风暴，将想法转化为设计

**触发方式：**

```markdown
/brainstorming
```

**描述：** 用于任何创造性工作之前（功能、组件、行为修改）

**核心原则：** 在提出设计方案并获得用户批准之前，**不要**开始实现

---

#### 2.6 requesting-code-review

**描述：** 请求代码审查

**触发方式：**

```markdown
/requesting-code-review
```

**描述：** Use when completing tasks, implementing major features, or before merging

**时机：**
- 子任务完成后
- 主要功能完成后
- 合并到 main 之前

---

#### 2.7 receiving-code-review

**描述：** 接收代码审查反馈

**触发方式：**

```markdown
/receiving-code-review
```

**描述：** Use when receiving code review feedback, before implementing suggestions

**核心原则：** 验证后再实现，不要盲目接受

---

#### 2.8 subagent-driven-development

**描述：** 子代理驱动开发

**触发方式：**

```markdown
/subagent-driven-development
```

**描述：** Use when executing implementation plans with independent tasks

**流程：** 每个任务派发新的子代理 + 两阶段审查（规范合规 → 代码质量）

---

#### 2.9 using-superpowers

**描述：** Superpowers 使用入门

**触发方式：**

```markdown
/using-superpowers
```

**描述：** Use when starting any conversation

---

#### 2.10 其他 Skills

| Skill | 触发命令 | 描述 |
|-------|---------|------|
| `executing-plans` | `/executing-plans` | 执行实现计划 |
| `finishing-a-development-branch` | `/finishing-a-development-branch` | 完成开发分支 |
| `dispatching-parallel-agents` | `/dispatching-parallel-agents` | 并行代理调度 |
| `using-git-worktrees` | `/using-git-worktrees` | 使用 Git Worktrees |
| `writing-skills` | `/writing-skills` | 编写新 Skills |

---

## Agents（代理）

**文件位置：** `~/.claude/agents/`

### 1. Code Reviewer

**文件：** `engineering-code-reviewer.md`

**触发方式：**

```markdown
Activate Code Reviewer and review this code
```

**描述：** Expert code reviewer who provides constructive, actionable feedback focused on correctness, maintainability, security, and performance

**关注点：**
- Correctness（正确性）
- Security（安全性）
- Maintainability（可维护性）
- Performance（性能）
- Testing（测试覆盖）

**优先级标记：**
- 🔴 blocker（必须修复）
- 🟡 suggestion（应该修复）
- 💭 nit（可选优化）

---

### 2. Frontend Developer

**文件：** `engineering-frontend-developer.md`

**触发方式：**

```markdown
Activate Frontend Developer and help me build this component
```

**描述：** Expert frontend developer specializing in modern web technologies, Vue/React/Angular frameworks

**技术栈支持：**
- Vue 3 ⭐
- React
- Angular
- Svelte

**核心能力：**
- 响应式设计
- 无障碍访问（WCAG 2.1 AA）
- Core Web Vitals 优化
- 像素级设计实现

---

### 3. Security Engineer

**文件：** `engineering-security-engineer.md`

**触发方式：**

```markdown
Activate Security Engineer and check for vulnerabilities
```

**描述：** Expert application security engineer specializing in threat modeling, vulnerability assessment, secure code review

**适用场景：**
- 新功能安全评估
- 权限系统审查
- 输入验证检查
- 认证/授权审计

**OWASP Top 10 + CWE Top 25** 合规检查

---

## 快速参考表

### Skills 命令速查

| 命令 | 用途 |
|------|------|
| `/systematic-debugging` | 遇到 bug 时使用 |
| `/test-driven-development` | 实现功能/修复前使用 |
| `/verification-before-completion` | 声称完成前使用 |
| `/writing-plans` | 多步骤任务前使用 |
| `/brainstorming` | 任何创造性工作前使用 |
| `/requesting-code-review` | 需要代码审查时 |
| `/receiving-code-review` | 收到审查反馈时 |
| `/subagent-driven-development` | 执行计划时 |

### Agents 激活速查

| Agent | 激活命令 |
|-------|---------|
| Code Reviewer | `Activate Code Reviewer and review this code` |
| Frontend Developer | `Activate Frontend Developer and help me build...` |
| Security Engineer | `Activate Security Engineer and check for vulnerabilities` |

---

## 配置文件位置

| 类型 | 路径 |
|------|------|
| 全局 Skills | `~/.claude/plugins/marketplaces/` |
| 全局 Agents | `~/.claude/agents/` |
| 项目 CLAUDE.md | `.claude/CLAUDE.md` |
| 官方插件 | `~/.claude/plugins/marketplaces/claude-plugins-official/` |

---

*本文件由 Claude Code 自动生成*
