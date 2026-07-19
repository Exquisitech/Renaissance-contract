# Renaissance Contract

This workspace contains the Soroban smart contracts for the Renaissance betting and NFT flow.

## Security review and verification

The repository now includes a structured security review package:

- Audit checklist: [docs/security-review-checklist.md](docs/security-review-checklist.md)
- Formal verification specs: [docs/formal-verification-specs.md](docs/formal-verification-specs.md)

## CI enforcement

The contract workflow enforces:

- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo audit`

## Unsafe code policy

The audited contract crates use `#![forbid(unsafe_code)]` and any future `unsafe` block must be documented with a justification before it is merged.
