# Security review checklist

This checklist is intended for pre-mainnet review of the Renaissance contracts.

## Checklist

- [x] Reentrancy: review the transfer and claim flows for reentrancy assumptions and ensure state is updated before external token transfers.
- [x] Overflow / underflow: use checked arithmetic for pool totals, payouts, and any arithmetic derived from user balances.
- [x] Access control: admin-only actions require authenticated principals and are gated by explicit authorization checks.
- [x] Front-running: value-changing actions are ordered around immutable state updates and settlement remains a dedicated oracle-controlled entry point.
- [x] Oracle manipulation: only the configured oracle can settle a match and the oracle address is replaceable only before settlement.

## Static analysis

- CI runs `cargo clippy --all-targets --all-features -- -D warnings`.
- CI runs `cargo audit` to surface dependency vulnerabilities before mainnet deployment.

## Unsafe code policy

The audited contract crates use `#![forbid(unsafe_code)]`. Any future `unsafe` block must be accompanied by a short justification describing why it is required and how it is bounded.
