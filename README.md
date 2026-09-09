# Turbin3 Week 2 – Escrow Program

Anchor escrow program for the Q3 2026 cohort.

## Overview

This is a classic maker/taker token escrow. The maker deposits token A into a PDA-controlled vault and specifies how much token B they want in return, along with an expiration timestamp. A taker can fill the order by sending the required amount of token B. If the order is not taken before the deadline, the maker can refund their tokens.

All four required instructions are implemented, plus the timed-escrow extension that uses the Clock sysvar.

## Instructions

| Instruction | Description |
|-------------|-------------|
| `make` | Maker creates the escrow, deposits token A into the vault, and sets the receive amount + expiration |
| `take` | Taker sends token B and receives token A. Fails if the escrow has expired |
| `refund` | Maker cancels the escrow and gets token A back (closes vault + escrow account) |
| `update` | Maker can change the `receive` amount while the escrow is still open |

### Timed Escrow

The `take` instruction reads the Clock sysvar:

```rust
let clock = Clock::get()?;
require!(clock.unix_timestamp <= self.escrow.expiration, ErrorCode::EscrowExpired);
```

If the current on-chain time is past the expiration, the transaction is rejected.

## Project Structure

```
programs/escrowq32026/
├── src/
│   ├── instructions/
│   │   ├── make.rs
│   │   ├── take.rs
│   │   ├── refund.rs
│   │   └── update.rs
│   ├── state.rs
│   ├── error.rs
│   ├── constants.rs
│   └── lib.rs
└── tests/
    └── mod.rs          # LiteSVM tests
```

## How to Build & Test

```bash
anchor build
cargo test --package escrowq32026 --test mod -- --nocapture
```

### Test Results

All three tests pass:

```
running 3 tests
test test_take_fails_when_expired ... ok
test test_make_and_refund ... ok
test test_make_update_and_take ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## Notes

- Vault program (Task 1) is completed in a separate repository.
- Stack size issues on the `Take` accounts struct were fixed by boxing the larger accounts.
- Tests are written with LiteSVM (no local validator required).
