# Contract Interface Reference

This document describes every public function exposed by the StellarAid smart
contracts. It is the authoritative source for off-chain integrators, backend
engineers, auditors, and front-end developers.

> **Source of truth.** The signatures in this document are taken from the
> on-chain contract source. When the contract changes, this document must
> be updated in the same PR.

## Conventions

- **Amounts** are in **stroops** for native XLM (1 XLM = 10,000,000 stroops) and
  in the token's smallest unit for SEP-41 tokens.
- **Timestamps** are Unix seconds (UTC).
- **Authorization**: every function explicitly states whether `require_auth`
  is called. Functions that do **not** require auth are read-only views.
- **Errors** are surfaced as `Error(Contract, #N)` in transaction results and
  decoded via the table in [Error Codes](#error-codes).

---

## Deployed Contracts

| Network  | Role             | Contract ID | Notes                 |
|----------|------------------|-------------|-----------------------|
| Testnet  | Campaign factory | _pending_   | See `deployments/`    |
| Testnet  | Sample campaign  | _pending_   | Created by factory    |
| Mainnet  | Campaign factory | _pending_   | Deployed via mainnet  |
| Mainnet  | Sample campaign  | _pending_   | Created by factory    |

The factory is the only top-level deployable; campaigns are created by
calling `deploy_campaign` (see [Factory](#factory-campaign-factory)).

---

## Campaign Contract (`CampaignContract`)

Implemented in `campaign/src/lib.rs` (`#[contractimpl]`). Storage and types in
`campaign/src/types.rs`.

### Constants

| Name            | Type | Value                         |
|-----------------|------|-------------------------------|
| `VERSION`       | u32  | `1`                           |
| `REFUND_WINDOW` | u64  | `2,592,000` (30 days, seconds) |
| `MAX_MILESTONES`| u32  | `5`                           |

---

### `initialize`

```rust
pub fn initialize(
    env: Env,
    creator: Address,
    goal_amount: i128,
    end_time: u64,
    accepted_assets: Vec<StellarAsset>,
    milestones: Vec<MilestoneData>,
    min_donation_amount: i128,
) -> Result<(), Error>
```

| Aspect          | Detail                                                                                  |
|-----------------|-----------------------------------------------------------------------------------------|
| Authorization   | `creator.require_auth()`                                                                |
| Description     | Create a new campaign. Admin is set to `creator`; rotatable via `set_admin`.            |
| Inputs          | `goal_amount` > 0 stroops; `end_time` strictly in the future; 1–5 milestones            |
| Outputs         | `Result<(), Error>` — `Ok(())` on success                                                |
| Events emitted  | `("campaign", "initialized")` → `CampaignInitializedEvent`                              |
| Errors          | `AlreadyInitialized`, `InvalidGoalAmount`, `InvalidEndTime`, `InvalidAssets`,           |
|                 | `InvalidAssetCode`, `InvalidMilestoneCount`, `InvalidMilestones`, `MilestoneMismatch`  |
| Notes           | Last milestone `target_amount` MUST equal `goal_amount`. Asset codes are 1–12 chars.    |

**Example**

```text
soroban contract invoke --id <CAMPAIGN_ID> -- \
    initialize \
    --creator GA…CREATOR \
    --goal_amount 1000000000 \
    --end_time 1735689600 \
    --accepted_assets '[{"asset_code":"XLM","issuer":null},{"asset_code":"USDC","issuer":"C…USDC"}]' \
    --milestones '[{...}]' \
    --min_donation_amount 1000000
```

A successful invocation publishes:
```rust
CampaignInitializedEvent { creator, goal_amount, end_time, asset_count, milestone_count, created_at_ledger }
```

---

### `set_admin`

```rust
pub fn set_admin(env: Env, new_admin: Address)
```

| Aspect        | Detail                                                              |
|---------------|---------------------------------------------------------------------|
| Authorization | **Both** `current_admin.require_auth()` and `new_admin.require_auth()` |
| Description   | Rotate administrative privileges. Old admin + new admin must both sign. |
| Events emitted| `("campaign", "admin_changed")` → `(old_admin, new_admin)`         |
| Errors        | `NotInitialized`, `Unauthorized`                                    |

---

### `freeze` / `unfreeze` / `upgrade`

```rust
pub fn freeze(env: Env)
pub fn unfreeze(env: Env)
pub fn upgrade(env: Env, new_wasm_hash: BytesN<32>)
```

| Aspect        | Detail                                                              |
|---------------|---------------------------------------------------------------------|
| Authorization | `admin.require_auth()`                                              |
| Description   | Toggle a contract-wide freeze or upgrade the WASM in place.          |
| Events emitted| `("campaign", "contract_frozen"|"contract_unfrozen"|"contract_upgraded")` |
| Errors        | `NotInitialized`, `Unauthorized`                                    |

When the contract is frozen, the following mutating operations are blocked
and return `Error::ContractFrozen` (80): `donate`, `claim_refund`,
`set_admin`, `end_campaign`, `cancel_campaign`, `extend_deadline`,
`release_milestone`, `release_milestone_multi_asset`.

---

### `donate`

```rust
pub fn donate(env: Env, donor: Address, amount: i128, asset: AssetInfo)
```

| Aspect        | Detail                                                                              |
|---------------|-------------------------------------------------------------------------------------|
| Authorization | `donor.require_auth()`                                                              |
| Description   | Send a donation. Updates aggregate donation record, per-asset totals, and unlocks milestones whose `target_amount` has been reached. |
| Events emitted| `("donation_received", contract_addr)` → `(donor, amount, asset_code, raised_total, ts)`; `("campaign", "campaign_goal_reached")` when raised ≥ goal; one `("milestone_unlocked", …)` per newly unlocked milestone. |
| Errors        | `ContractFrozen`, `NotInitialized`, `CampaignNotActive`, `AssetNotAccepted`, `DonationTooSmall`, `Overflow`, `ReentrantCall` |

---

### `get_total_raised`

```rust
pub fn get_total_raised(env: Env) -> i128
```

Read-only view. Returns the global `TotalRaised` counter across all assets.

---

### `get_raised_per_asset`

```rust
pub fn get_raised_per_asset(env: Env) -> Vec<(AssetInfo, i128)>
```

Read-only view. Returns `[(asset, amount)]` for every accepted asset.

---

### `get_donor_record` / `get_donor_asset_breakdown`

```rust
pub fn get_donor_record(env: Env, donor: Address) -> Option<DonorRecord>
pub fn get_donor_asset_breakdown(env: Env, donor: Address) -> Vec<(AssetInfo, i128)>
```

Read-only views. Return the aggregate donor record or the asset-level
contribution breakdown. Both are `None` / empty for first-time donors.

---

### `is_refund_eligible`

```rust
pub fn is_refund_eligible(env: Env, donor: Address) -> bool
```

Read-only view. Returns `true` when:

1. The campaign status is terminal (`Ended` or `Cancelled`).
2. For `Ended`: no milestone has been released yet.
3. `now <= campaign.end_time + REFUND_WINDOW` (30 days).
4. The donor has not already claimed a refund.

---

### `claim_refund`

```rust
pub fn claim_refund(env: Env, donor: Address)
```

| Aspect        | Detail                                                                                  |
|---------------|-----------------------------------------------------------------------------------------|
| Authorization | `donor.require_auth()`                                                                  |
| Description   | Pro-rata refund across every asset the donor contributed in.                            |
| Events emitted| One `("campaign", "asset_refund")` per asset transferred, then a single               |
|               | `("campaign", "refund_claimed")` event.                                                  |
| Errors        | `ContractFrozen`, `NotInitialized`, `NoDonorRecord`, `RefundNotPermitted`,              |
|               | `RefundWindowClosed`, `RefundAlreadyClaimed`, `InsufficientContractBalance`            |

Formula:

```text
refund_amount(asset) = (donor_asset_amount * (raised − released)) / raised
```

---

### `end_campaign` / `cancel_campaign` / `extend_deadline`

```rust
pub fn end_campaign(env: Env)
pub fn cancel_campaign(env: Env)
pub fn extend_deadline(env: Env, new_end_time: u64)
```

| Aspect        | Detail                                                              |
|---------------|---------------------------------------------------------------------|
| Authorization | `creator.require_auth()`                                            |
| Events emitted| `("campaign", "campaign_ended"|"campaign_cancelled"|"deadline_extended")` |
| Errors        | `NotInitialized`, `InvalidCampaignTransition`, `InvalidEndTime`     |

`extend_deadline` requires `new_end_time > env.ledger().timestamp()`.

---

### `get_campaign_status`

```rust
pub fn get_campaign_status(env: Env) -> CampaignStatusResponse
```

Read-only view. Returns `{ status: CampaignStatus, days_remaining: i64 }`.
`days_remaining` is positive while the deadline is in the future and
negative after it has passed.

---

### `release_milestone` / `release_milestone_multi_asset`

```rust
pub fn release_milestone(env: Env, milestone_index: u32, recipient: Address)
pub fn release_milestone_multi_asset(env: Env, milestone_index: u32, recipient: Address)
```

| Aspect        | Detail                                                                                  |
|---------------|-----------------------------------------------------------------------------------------|
| Authorization | `creator.require_auth()`                                                                |
| Description   | Release a single milestone's proportional share of each accepted asset.                 |
| Events emitted| One `("milestone_released", contract)` → `(index, amount, asset_code, recipient, ts)` per asset. |
| Errors        | `NotInitialized`, `ContractFrozen`, `MilestoneNotFound`, `InvalidMilestoneTransition`, |
|               | `MilestoneAlreadyReleased`, `PreviousMilestoneNotReleased`, `InvalidRecipient`,          |
|               | `InsufficientContractBalance`, `NothingToRelease`, `ReentrantCall`                      |

Releases are **strictly sequential**: every milestone before `milestone_index`
must be in the `Released` state, and the milestone itself must currently be
`Unlocked` (auto-unlocked as reached by donations). The multi-asset variant
enforces the Checks-Effects-Interactions pattern and clamps releases to the
contract's actual on-chain balance.

---

### `get_milestone_view` / `get_all_milestones`

```rust
pub fn get_milestone_view(env: Env, index: u32) -> MilestoneData
pub fn get_all_milestones(env: Env) -> Vec<MilestoneView>
```

Read-only views.

* `get_milestone_view` returns the raw stored `MilestoneData`.
* `get_all_milestones` returns `MilestoneView` records — the raw data plus
  `pending_release`, `is_fully_released`, and `is_next_pending` (see
  `campaign/src/views.rs`).

---

### `hello` / `version` / `get_admin`

```rust
pub fn hello(env: Env) -> Symbol              // returns Symbol "campaign"
pub fn version() -> u32                       // returns VERSION (1)
pub fn get_admin(env: Env) -> Option<Address> // current admin
```

Diagnostic helpers, all read-only.

---

## Factory (`CampaignFactory`)

Implemented in `factory/src/lib.rs`.

### `initialize`

```rust
pub fn initialize(env: Env, admin: Address, treasury: Address)
```

| Aspect        | Detail                                                              |
|---------------|---------------------------------------------------------------------|
| Authorization | `admin.require_auth()`                                              |
| Description   | Configure the factory once. Stores admin, treasury, empty registry. |
| Errors        | `panic!("already initialized")` if called twice                     |

### `update_wasm_hash` / `get_wasm_hash`

```rust
pub fn update_wasm_hash(env: Env, new_hash: BytesN<32>)
pub fn get_wasm_hash(env: Env) -> Option<BytesN<32>>
```

Admin-only WASM hash management. `update_wasm_hash` publishes a
`("wasm_upd",)` event with the new hash.

### `set_deployment_fee` / `get_deployment_fee`

```rust
pub fn set_deployment_fee(env: Env, fee: i128)
pub fn get_deployment_fee(env: Env) -> i128
```

Admin-only. Default fee = `0` (free deployments).

### `deploy_campaign`

```rust
pub fn deploy_campaign(
    env: Env,
    creator: Address,
    xlm_token: Address,
    params: CampaignParams,
) -> Address
```

| Aspect        | Detail                                                                                  |
|---------------|-----------------------------------------------------------------------------------------|
| Authorization | `creator.require_auth()`                                                                |
| Description   | Deploy a new campaign contract from the stored WASM hash. If `deployment_fee > 0`, the caller pays the fee in XLM to the treasury. |
| Outputs       | The contract address of the new campaign.                                              |
| Events emitted| `("campaign_deployed", creator)` → `deployed_address`                                  |
| Errors        | `panic!("wasm hash not set")`, `panic!("treasury not set")`                             |
| Parameters    | `CampaignParams { creator, salt: BytesN<32> }`                                          |

The `salt` is what allows a single creator to deploy multiple campaigns at
deterministic addresses.

### `get_all_campaigns` / `get_campaigns_by_creator`

Read-only views of the registry (`Vec<(Address, Address)>` of
`(creator, contract)` pairs).

### `get_admin` / `get_campaign_count`

Read-only views.

### `set_treasury`

Admin-only. Publishes `("treasury_s",)` event with the new address.

---

## Token Bridge (`TokenBridgeContract`)

Implemented in `token-bridge/src/lib.rs`. Minimal placeholder contract
handling native / wrapped asset bridging.

```rust
pub fn hello(env: Env) -> Symbol  // returns Symbol "token_bridge"
```

No state-changing operations are exposed yet.

---

## Error Codes

The full enum lives in `campaign/src/types.rs`. Codes are part of the
contract ABI and are **never renumbered** — only new ones are appended.

| Code | Variant                         | Where raised                       |
|------|---------------------------------|------------------------------------|
| 1    | `AlreadyInitialized`            | `initialize`                       |
| 2    | `NotInitialized`                | every function requiring init      |
| 3    | `Unauthorized`                  | missing `require_auth`             |
| 4    | `CampaignEnded`                 | deadline has passed                |
| 5    | `CampaignNotActive`             | donate on non-Active campaign      |
| 6    | `AssetNotAccepted`              | donation in unlisted asset         |
| 7    | `DonationTooSmall`              | below `min_donation_amount`        |
| 8    | `MilestoneNotFound`             | out-of-range or missing index      |
| 9    | `MilestoneNotUnlocked`          | release of a Locked milestone      |
| 10   | `PreviousMilestoneNotReleased`  | non-sequential milestone release   |
| 11   | `CannotCancelWithFunds`         | cancel while holding funds         |
| 12   | `RefundWindowClosed`            | > 30 days past deadline            |
| 13   | `InvalidGoalAmount`             | non-positive goal                  |
| 14   | `InvalidEndTime`                | deadline ≤ now                     |
| 15   | `InvalidMilestones`             | non-ascending milestone targets    |
| 16   | `InsufficientContractBalance`   | release/refund exceeds balance     |
| 17   | `Overflow`                      | checked-arithmetic overflow        |
| 18   | `InvalidAssets`                 | empty `accepted_assets`            |
| 19   | `InvalidAssetCode`              | asset code > 12 chars / empty      |
| 20   | `MilestoneMismatch`             | last milestone target ≠ goal       |
| 21   | `InvalidMilestoneCount`         | not in [1, MAX_MILESTONES]         |
| 22   | `InvalidCampaignTransition`     | illegal status transition          |
| 23   | `InvalidMilestoneTransition`    | illegal milestone transition       |
| 24   | `GoalNotReached`                | premature `Ended`                  |
| 25   | `InvalidStorageValue`           | corrupted storage read             |
| 26   | `StorageWriteError`             | failed write                       |
| 30   | `InvalidRecipient`              | recipient == contract              |
| 31   | `MissingIssuerAddress`          | unresolvable xfer                  |
| 32   | `ZeroReleaseAmount`             | all amounts rounded to 0           |
| 33   | `NothingToRelease`              | already fully released             |
| 34   | `MilestoneReleasedExceedsTarget`| invariant broken                   |
| 40   | `MilestoneAlreadyReleased`      | double release                     |
| 41   | `UnreleasedMilestonesExist`     | premature closure                  |
| 50   | `RefundNotPermitted`            | non-terminal campaign on refund    |
| 51   | `NoDonorRecord`                 | donor never donated                |
| 52   | `RefundAlreadyClaimed`          | double refund                      |
| 60   | `ReentrantCall`                 | re-entry guard tripped             |
| 70   | `InvalidAmount`                 | non-positive amount                |
| 80   | `ContractFrozen`                | mutating op while frozen           |

For HTTP-integration guidance see
[`docs/backend-integration.md`](./backend-integration.md).

---

## Events

All events are published via the host's `env.events().publish`. Full topic
schemes are in [`docs/events.md`](./events.md). Off-chain consumers parse
event XDR using the topic tuples and the data payload listed below.

| Topic                                                        | Function          | Payload                                                                     |
|--------------------------------------------------------------|-------------------|-----------------------------------------------------------------------------|
| `("campaign", "initialized")`                                | `initialize`      | `CampaignInitializedEvent`                                                 |
| `("campaign", "campaign_goal_reached")`                      | `donate`          | `i128` (new raised total)                                                   |
| `("donation_received", contract_addr)`                       | `donate`          | `(Address, i128, String, i128, u64)` — `(donor, amount, asset_code, total, ts)` |
| `("campaign", "asset_refund")`                               | `claim_refund`    | `(Address, Address, i128)` — `(donor, asset_addr, amount)`                 |
| `("campaign", "refund_claimed")`                             | `claim_refund`    | `(Address, i128)` — `(donor, total_donated)`                                |
| `("campaign", "campaign_ended")`                             | `end_campaign`    | `()`                                                                        |
| `("campaign", "campaign_cancelled")`                         | `cancel_campaign` | `Address` (creator)                                                         |
| `("campaign", "deadline_extended")`                          | `extend_deadline` | `(Address, u64, u64)`                                                       |
| `("campaign", "admin_changed")`                              | `set_admin`       | `(Address, Address)`                                                        |
| `("campaign", "contract_frozen")`                            | `freeze`          | `(Address, u64)`                                                            |
| `("campaign", "contract_unfrozen")`                          | `unfreeze`        | `(Address, u64)`                                                            |
| `("campaign", "contract_upgraded")`                          | `upgrade`         | `(Address, BytesN<32>, u64)`                                                |
| `("milestone_unlocked", contract_addr)`                      | `donate`          | `(u32, i128, i128)`                                                         |
| `("milestone_released", contract_addr)`                      | release           | `(u32, i128, String, Address, u64)` — `(index, amount, asset_code, recipient, ts)` |
| `("wasm_upd",)` (factory)                                    | `update_wasm_hash`| `BytesN<32>`                                                                |
| `("fee_set",)` (factory)                                     | `set_deployment_fee` | `i128`                                                                   |
| `("treasury_s",)` (factory)                                  | `set_treasury`    | `Address`                                                                   |
| `("campaign_deployed", creator)` (factory)                   | `deploy_campaign` | `Address` (deployed contract)                                               |
