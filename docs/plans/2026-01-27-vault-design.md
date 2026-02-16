# Solana DeFi Vault - 设计文档

## 项目概述

去中心化资产 Vault，用户存入 SPL Token，Vault 自动管理资金，支持收益分配和管理员策略。

- **框架**: Anchor
- **许可证**: MIT
- **开源**: 是

## 核心设计决策

| 决策项 | 选择 |
|--------|------|
| Token 支持 | 多 Token（白名单） |
| 份额凭证 | Vault Shares Token (如 vUSDC) |
| 策略模式 | 先手动，后期可扩展为自动 |
| 权限模式 | 分级权限 (Owner + Operator) |
| 收益分配 | 自动复利 |
| 费用结构 | 绩效费 + 提款费 |
| 提款机制 | 按流动性即时提款 + 队列 |
| 安全功能 | 完整安全套件 |

---

## 整体架构

```
┌─────────────────────────────────────────────────────────────┐
│                     Solana DeFi Vault                       │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐     │
│  │ USDC Vault  │    │ SOL Vault   │    │ USDT Vault  │     │
│  │   Pool      │    │   Pool      │    │   Pool      │     │
│  └──────┬──────┘    └──────┬──────┘    └──────┬──────┘     │
│         │                  │                  │             │
│         ▼                  ▼                  ▼             │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐     │
│  │ vUSDC Share │    │ vSOL Share  │    │ vUSDT Share │     │
│  │   Token     │    │   Token     │    │   Token     │     │
│  └─────────────┘    └─────────────┘    └─────────────┘     │
│                                                             │
├─────────────────────────────────────────────────────────────┤
│  权限层：  Owner (最高权限)  ◄──►  Operator (日常操作)      │
├─────────────────────────────────────────────────────────────┤
│  安全层：  暂停开关 │ 提款上限 │ 白名单 │ 紧急提款          │
└─────────────────────────────────────────────────────────────┘
```

**核心设计理念：**
- 一个 Token 一个 Vault Pool：每种白名单 Token 独立管理
- Shares Token 代表份额：用户存入 USDC，获得 vUSDC
- 自动复利：Vault 总资产增加 → Shares 升值
- 份额计算：`shares = deposit_amount * total_shares / total_assets`

---

## 账户结构 (Anchor PDA)

```rust
/// 全局配置 - 单例
#[account]
pub struct VaultConfig {
    pub owner: Pubkey,            // 最高权限
    pub operator: Pubkey,         // 日常操作员
    pub is_paused: bool,          // 全局暂停开关
    pub performance_fee_bps: u16, // 绩效费 (如 2000 = 20%)
    pub withdrawal_fee_bps: u16,  // 提款费 (如 50 = 0.5%)
    pub fee_receiver: Pubkey,     // 费用接收地址
    pub bump: u8,
}

/// 单个 Token 的 Vault Pool
#[account]
pub struct VaultPool {
    pub config: Pubkey,              // 指向 VaultConfig
    pub token_mint: Pubkey,          // 底层 Token (如 USDC)
    pub shares_mint: Pubkey,         // Shares Token (如 vUSDC)
    pub token_vault: Pubkey,         // 存放资产的 Token Account
    pub total_assets: u64,           // 总资产（含已部署 + 闲置）
    pub total_shares: u64,           // 已发行 Shares 总量
    pub daily_withdrawal_limit: u64, // 每日提款上限
    pub withdrawn_today: u64,        // 今日已提款金额
    pub last_withdrawal_day: i64,    // 上次提款日期
    pub is_active: bool,             // 该池是否激活
    pub bump: u8,
}

/// 用户提款请求（队列模式）
#[account]
pub struct WithdrawalRequest {
    pub user: Pubkey,
    pub vault_pool: Pubkey,
    pub shares_amount: u64,  // 请求赎回的 Shares
    pub requested_at: i64,   // 请求时间
    pub bump: u8,
}
```

---

## 核心指令

### 管理指令 (Owner)

| 指令 | 描述 |
|------|------|
| `initialize_config` | 初始化全局配置 |
| `update_config` | 更新费用、权限等 |
| `transfer_ownership` | 转移 Owner 权限 |
| `set_operator` | 设置 Operator |

### Pool 管理 (Owner)

| 指令 | 描述 |
|------|------|
| `create_vault_pool` | 创建新 Token 的 Vault Pool |
| `set_pool_status` | 激活/停用某个 Pool |
| `set_withdrawal_limit` | 设置提款上限 |

### 运营指令 (Operator)

| 指令 | 描述 |
|------|------|
| `inject_yield` | 注入收益（自动复利） |
| `withdraw_for_strategy` | 提取资金用于策略部署 |
| `return_from_strategy` | 策略资金回流 |
| `process_withdrawal_queue` | 处理提款队列 |

### 用户指令

| 指令 | 描述 |
|------|------|
| `deposit` | 存入 Token → 获得 Shares |
| `withdraw` | 赎回 Shares → 取回 Token (即时) |
| `request_withdrawal` | 发起提款请求（队列模式） |
| `cancel_withdrawal` | 取消提款请求 |

### 紧急指令 (Owner)

| 指令 | 描述 |
|------|------|
| `pause` / `unpause` | 暂停/恢复所有操作 |
| `emergency_withdraw` | 紧急提取全部资金 |

### 核心计算逻辑

**Deposit:**
```
shares_to_mint = deposit_amount * total_shares / total_assets
// 首次存款时 1:1 铸造
```

**Withdraw:**
```
assets_to_return = shares_amount * total_assets / total_shares
fee = assets_to_return * withdrawal_fee_bps / 10000
user_receives = assets_to_return - fee
```

---

## 安全机制

### 1. 暂停机制 (Pause)
- 全局暂停：`is_paused` in VaultConfig
- 单池暂停：`is_active` in VaultPool
- 暂停后：仅允许 `emergency_withdraw`

### 2. 提款上限
- 每日限额：`daily_withdrawal_limit`
- 滚动重置：每天 UTC 0:00 重置
- 超限：自动进入提款队列

### 3. 权限检查
- `#[access_control]` 宏验证签名者
- Owner：最高权限操作
- Operator：日常运营操作
- 任何用户：deposit/withdraw

### 4. 数值安全
- `checked_math`：防止溢出
- 最小存款限制：防止精度攻击
- Shares 计算：首次存款 1:1，避免除零

### 5. 紧急提款
- 仅 Owner 可执行
- 将全部资产转至指定安全地址
- 自动暂停所有操作

---

## 项目结构

```
solana-defi-vault/
├── Anchor.toml
├── Cargo.toml
├── LICENSE                  # MIT
├── README.md
│
├── programs/
│   └── vault/
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── state/
│           │   ├── mod.rs
│           │   ├── config.rs
│           │   ├── pool.rs
│           │   └── withdrawal.rs
│           ├── instructions/
│           │   ├── mod.rs
│           │   ├── admin/
│           │   │   ├── initialize.rs
│           │   │   ├── create_pool.rs
│           │   │   └── emergency.rs
│           │   ├── operator/
│           │   │   ├── inject_yield.rs
│           │   │   └── process_queue.rs
│           │   └── user/
│           │       ├── deposit.rs
│           │       └── withdraw.rs
│           ├── errors.rs
│           └── utils.rs
│
├── tests/
│   ├── vault.ts
│   └── utils.ts
│
└── app/                     # 可选：前端/SDK
    └── ...
```

---

## 后续扩展

1. **自动化策略集成**
   - 接入 Marinade (stSOL)
   - 接入 Kamino (借贷)
   - 策略接口抽象

2. **治理升级**
   - 多签集成 (Squads Protocol)
   - DAO 投票决策

3. **前端 SDK**
   - TypeScript SDK
   - React Hooks
