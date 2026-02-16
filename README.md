# Solana DeFi Vault

A decentralized asset vault on Solana that supports multi-token deposits, automatic yield compounding, and flexible withdrawal mechanisms.

## Features

- **Multi-Token Support**: Whitelist-based token management with separate vault pools
- **Vault Shares Token**: ERC-4626 style shares representing user deposits (e.g., vUSDC for USDC)
- **Auto-Compounding**: Yields are automatically compounded into the vault
- **Tiered Access Control**: Owner (admin) and Operator (daily operations) roles
- **Flexible Withdrawals**: Instant withdrawals with liquidity, queue-based when funds are deployed
- **Security Features**: Pause mechanism, daily withdrawal limits, emergency withdraw

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Solana DeFi Vault                       │
├─────────────────────────────────────────────────────────────┤
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
├─────────────────────────────────────────────────────────────┤
│  Roles:  Owner (highest privilege)  ◄──►  Operator (ops)   │
├─────────────────────────────────────────────────────────────┤
│  Security:  Pause │ Withdrawal Limits │ Emergency Withdraw │
└─────────────────────────────────────────────────────────────┘
```

## Project Structure

```
solana-defi-vault/
├── programs/vault/src/
│   ├── lib.rs                 # Program entry point
│   ├── state/                 # Account structures
│   │   ├── config.rs          # VaultConfig (global settings)
│   │   ├── pool.rs            # VaultPool (per-token vault)
│   │   └── withdrawal.rs      # WithdrawalRequest (queue)
│   ├── instructions/
│   │   ├── admin/             # Owner-only instructions
│   │   ├── operator/          # Operator instructions
│   │   └── user/              # User instructions
│   ├── errors.rs              # Custom error codes
│   └── utils.rs               # Helper functions
├── tests/
│   └── vault.ts               # Integration tests
├── postman/
│   ├── solana-vault-api-tests.postman_collection.json  # API test collection
│   └── solana-vault-devnet.postman_environment.json    # Devnet environment
├── .github/workflows/
│   └── ci.yml                 # CI/CD pipeline (build + test + API tests)
└── docs/plans/
    └── 2026-01-27-vault-design.md
```

## Instructions

### Admin (Owner Only)

| Instruction | Description |
|-------------|-------------|
| `initialize` | Initialize global vault configuration |
| `update_config` | Update fees, operator, fee receiver |
| `create_pool` | Create a new vault pool for a token |
| `pause` / `unpause` | Pause/resume all operations |
| `set_pool_status` | Activate/deactivate a specific pool |
| `set_withdrawal_limit` | Set daily withdrawal limit |
| `emergency_withdraw` | Emergency fund extraction |
| `transfer_ownership` | Transfer owner role |
| `close_pool` | Close an empty, inactive pool (reclaim rent) |
| `close_withdrawal_request` | Close processed withdrawal requests |

### Operator

| Instruction | Description |
|-------------|-------------|
| `inject_yield` | Inject yield (auto-compounds, takes performance fee) |
| `withdraw_for_strategy` | Deploy funds to external strategies |
| `return_from_strategy` | Return funds from strategies |
| `process_withdrawal_queue` | Process pending withdrawal requests |

### User

| Instruction | Description |
|-------------|-------------|
| `deposit` | Deposit tokens, receive shares |
| `withdraw` | Burn shares, receive tokens (instant if liquid) |
| `request_withdrawal` | Request withdrawal (enters queue) |
| `cancel_withdrawal` | Cancel pending withdrawal request |

### View Functions

| Instruction | Description |
|-------------|-------------|
| `preview_deposit` | Preview shares to receive for a deposit |
| `preview_withdraw` | Preview assets to receive for a withdrawal |
| `get_pool_info` | Get pool statistics and share price |

## Fees

- **Performance Fee**: Charged on yield injection (default: 20%)
- **Withdrawal Fee**: Charged on withdrawals (default: 0.5%)

## CI/CD

[![Solana Vault CI](https://github.com/binfengke/Solana-Defi-Vault/actions/workflows/ci.yml/badge.svg)](https://github.com/binfengke/Solana-Defi-Vault/actions/workflows/ci.yml)

Automated pipeline via GitHub Actions (`.github/workflows/ci.yml`):

| Stage | What it does |
|-------|-------------|
| **Build** | Install Rust, Solana CLI, Anchor → `anchor build` |
| **Test** | Run integration tests → `anchor test` |
| **API Tests** | Run Postman collection against Devnet via Newman |
| **Report** | Generate HTML test report as build artifact |

Triggered on every `push` and `pull_request` to `main`.

## API Testing (Postman)

The `postman/` directory contains a Postman collection for testing Solana JSON-RPC endpoints against the vault contract.

### Requests

| Category | Request | Validates |
|----------|---------|-----------|
| Health Check | Get Cluster Health | RPC node is responsive |
| Health Check | Get Latest Blockhash | Network is producing blocks |
| Wallet & Balance | Get SOL Balance | Wallet balance query, lamport-to-SOL conversion |
| Wallet & Balance | Get Token Accounts by Owner | SPL token holdings for a wallet |
| Transaction | Get Transaction by Signature | Tx status, fee, balance changes |
| Transaction | Get Recent Signatures | Wallet transaction history |
| Vault Contract | Get Program Account Info | Contract is deployed and executable |
| Vault Contract | Get Program Accounts (All Pools) | Enumerate all vault pool accounts |

Each request includes **automated test scripts** (Chai assertions) that validate response structure, data types, and business logic.

### Run locally

```bash
# Install Newman (Postman CLI runner)
npm install -g newman

# Run collection against Devnet
newman run postman/solana-vault-api-tests.postman_collection.json \
  --environment postman/solana-vault-devnet.postman_environment.json
```

### Import into Postman

1. Open Postman → Import → Upload Files
2. Select `postman/solana-vault-api-tests.postman_collection.json`
3. Import `postman/solana-vault-devnet.postman_environment.json` as environment
4. Update `wallet_address` and `tx_signature` variables with your values

## Getting Started

### Prerequisites

- Rust 1.70+
- Solana CLI 1.18+
- Anchor 0.30+
- Node.js 18+

### Build

```bash
anchor build
```

### Test

```bash
anchor test
```

### Deploy

```bash
# Devnet
anchor deploy --provider.cluster devnet

# Mainnet
anchor deploy --provider.cluster mainnet
```

## Security Considerations

1. **Pause Mechanism**: Owner can pause all operations in emergencies
2. **Daily Limits**: Configurable daily withdrawal limits per pool
3. **Tiered Access**: Separation of owner and operator privileges
4. **Checked Math**: All arithmetic uses checked operations to prevent overflow
5. **Minimum Deposit**: Prevents precision/rounding attacks

## Future Enhancements

- [ ] Automated strategy integration (Marinade, Kamino)
- [ ] Multisig support (Squads Protocol)
- [ ] DAO governance
- [x] TypeScript SDK
- [ ] React hooks for frontend integration

## SDK Usage

The project includes a TypeScript SDK for easy frontend integration:

```typescript
import { VaultClient } from "@solana-defi-vault/sdk";
import { Connection, PublicKey } from "@solana/web3.js";

// Initialize client
const connection = new Connection("https://api.devnet.solana.com");
const client = new VaultClient(connection);

// Get pool info
const poolInfo = await client.getPoolInfo(tokenMint);
console.log("Share Price:", poolInfo.sharePrice);
console.log("TVL:", poolInfo.tvl.toString());

// Get user position
const position = await client.getUserPosition(tokenMint, userPublicKey);
console.log("Shares:", position.sharesBalance.toString());
console.log("Estimated Value:", position.estimatedValue.toString());

// Simulate deposit
const preview = await client.simulateDeposit(tokenMint, depositAmount);
console.log("Shares to receive:", preview.sharesToReceive.toString());

// Build deposit instruction
const instructions = await client.buildDepositInstruction(
  userPublicKey,
  tokenMint,
  depositAmount
);
```

## Events

All state-changing instructions emit events for easy tracking:

| Event | Description |
|-------|-------------|
| `VaultInitialized` | Emitted when vault is initialized |
| `PoolCreated` | Emitted when a new pool is created |
| `Deposit` | Emitted on user deposits |
| `Withdrawal` | Emitted on user withdrawals |
| `YieldInjected` | Emitted when yield is added |
| `VaultPaused` / `VaultUnpaused` | Emitted on pause state changes |
| `EmergencyWithdrawal` | Emitted on emergency fund extraction |

## License

MIT License - see [LICENSE](LICENSE)
