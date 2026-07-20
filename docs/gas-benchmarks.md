# Gas / resource benchmarks

Soroban transaction fees are driven by two host-metered resources per
invocation: CPU instructions and memory bytes. This document tracks the
budget assumed for each benchmarked public function, backed by the
`gas_bench` test modules in `counter/src/lib.rs` and `betting/src/lib.rs`
(run via `cargo test --release -p renaissance-counter gas_bench` and
`cargo test --release -p renaissance-betting gas_bench`). CI runs both as
a required step in `.github/workflows/contracts.yml` and fails the build if
a function's measured cost exceeds its budget.

## How measurement works

Each benchmark test resets the environment's budget tracker immediately
before the call under test (`env.budget().reset_unlimited()`), invokes the
function once, then reads back `env.budget().cpu_instruction_cost()` and
`env.budget().memory_bytes_cost()`. Because Soroban's cost model is a fixed
instruction-counting scheme (not wall-clock time), these numbers are
deterministic across machines and CI runs — the same contract call always
reports the same cost.

## Target vs. actual

The "Actual" column is the measured cost as of this PR. The "Budget"
column is the threshold enforced in `gas_bench` — set with headroom above
the measured baseline so routine SDK/dependency bumps don't cause CI
flakes, while still catching real regressions (e.g. an unbounded storage
read creeping back in).

> **Note on "Actual" values:** this PR was authored and locally verified on
> a Windows host where both the `x86_64-pc-windows-gnu` and
> `x86_64-pc-windows-msvc` Rust toolchains hit environment-specific linker
> issues (missing `dlltool.exe` for `-gnu`; a broken/incomplete MSVC "C++
> build tools" install for `-msvc`) that blocked running `cargo test`
> locally — unrelated to this repo's code. The "Budget" thresholds below
> were set conservatively (based on the shape of each function's storage
> and arithmetic operations) rather than tuned to a locally-measured
> number. **The `Gas budget benchmarks` CI job on this PR is the first
> real run** — the "Actual" column will be filled in from that run's
> output before merge, and thresholds tightened to match.

| Contract  | Function       | Budget (CPU insns) | Budget (mem bytes) | Actual (CPU insns) | Actual (mem bytes) |
|-----------|----------------|--------------------:|--------------------:|--------------------:|--------------------:|
| counter   | `increment`    | 400,000             | 40,000               | _pending CI run_    | _pending CI run_    |
| counter   | `get_count`    | 100,000             | 15,000               | _pending CI run_    | _pending CI run_    |
| betting   | `place_bet`    | 3,000,000           | 300,000              | _pending CI run_    | _pending CI run_    |
| betting   | `settle_bet`   | 1,500,000           | 150,000              | _pending CI run_    | _pending CI run_    |
| betting   | `claim_payout` | 3,000,000           | 300,000              | _pending CI run_    | _pending CI run_    |
| betting   | `refund_bet`   | 3,000,000           | 300,000              | _pending CI run_    | _pending CI run_    |
| betting   | `get_bet`      | 500,000             | 60,000                | _pending CI run_    | _pending CI run_    |
| betting   | `get_match`    | 500,000             | 60,000                | _pending CI run_    | _pending CI run_    |

`counter` and `betting` were chosen because they're the two contracts this
issue names directly (`get_count`, `get_bet`). `oracle` and `vault` already
had `testutils` wired up correctly and their own test suites; `rewards` and
`player-nft` had the same missing-`testutils` gap fixed by this PR (see
below) but aren't benchmarked here — extending `gas_bench` coverage to all
six contracts is a reasonable follow-up.

## Optimizations applied

### `counter`: `COUNTER` moved from persistent to temporary storage

`increment`/`decrement`/`get_count` now read and write `COUNTER` via
`env.storage().temporary()` instead of `.persistent()`. Temporary entries
are cheaper for the host to read/write because they carry no long-term
rent/TTL-extension accounting. This is safe specifically because the
counter is a demo contract with no value at stake: if the entry's TTL
lapses from inactivity, the count silently resets to 0, which is an
acceptable outcome.

### `betting`: `get_bet` stays on persistent storage (intentionally not optimized)

The issue asked to evaluate `get_count` and `get_bet` for a move to
temporary storage "where safe." For `get_bet`, it is **not** safe: the
underlying `Bet` record holds real transferred funds (`amount`), a
`claimed` flag gating a second withdrawal, and the paid-out `payout`. If a
temporary entry's TTL lapsed before a bettor called `claim_payout` or
`refund_bet`, the record — and the bettor's ability to recover their
funds — would be gone. `Bet` stays in persistent storage; this is
documented directly in `betting/src/lib.rs`'s module doc comment.

### `betting`: removed the unbounded `bettors` list from `MatchStats`

`MatchStats.bettors: Vec<Address>` was appended to and the *entire*
`MatchStats` record rewritten on every single `place_bet` call, but the
field was never read anywhere (not even by `claim_payout`, which only
reads `pools`). That made `place_bet`'s storage cost grow linearly with
the number of bettors already on a match — the same class of problem as
Soroban's well-known "push-payment" anti-pattern, just on the write side
of an aggregate collection instead of a payout loop. The field has been
removed, turning that per-call cost back into a constant. `oracle` and
`rewards` were also audited for similar unbounded per-write collections;
their `Vec<Address>` fields (`oracles`, `confirmations`) are
admin-/multisig-controlled and bounded by a small, slow-growing set, so no
change was needed there.

## Fixing `cargo test` across the workspace

While wiring up `gas_bench`, `cargo test --release -p renaissance-counter`
failed to compile: `counter`'s (and `betting`'s, `rewards`'s, and
`player-nft`'s) `Cargo.toml` never declared the `soroban-sdk` `testutils`
feature in `[dev-dependencies]`, so `Env::mock_all_auths`,
`Address::generate`, `env.register_contract`, etc. were unavailable —
meaning **none of these four crates' existing `#[cfg(test)]` modules had
ever actually compiled**. This wasn't caught before because
`.github/workflows/contracts.yml` only ran `cargo check`, never
`cargo test`. `oracle` and `vault` already had the correct
`[dev-dependencies] soroban-sdk = { workspace = true, features =
["testutils"] }` pattern; this PR applies the same pattern to the other
four crates so their test suites (and this PR's new `gas_bench` modules)
can actually run.
