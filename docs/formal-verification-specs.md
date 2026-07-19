# Formal verification specs

## Betting contract: payout correctness

For a settled match, let:

- $w$ be the winning outcome pool
- $t$ be the total pool across all outcomes
- $b$ be the bettor's stake for the winning outcome

The payout for a winning bet is:

$$
\text{payout} = b + \left\lfloor \frac{b \cdot (t - w)}{w} \right\rfloor
$$

The implementation must ensure:

1. The payout is computed only for bets on the winning outcome.
2. The winning pool is strictly positive before division.
3. The payout is computed with checked arithmetic to avoid overflow.
4. The bet is marked as claimed and the transfer amount equals the computed payout.

## Player NFT contract: ownership invariant

The ownership invariant is:

- Each token has exactly one current owner at any time.
- The owner identity returned by the contract must match the most recent successful transfer or mint event.
- Ownership transitions are monotonic with respect to the authorized transfer flow.

The current implementation is intentionally a contract boundary with no transfer logic yet; the invariant is documented here so future NFT ownership logic can be verified against it.
