# openOODA v0.22.0-alpha — CHS M1–M5 (frontend self-host)

## Claim (truthful)

**CHS frontend self-host is green:** native `oodac` matches stage-0 token dumps;
two-generation rebuild digests match; smoke C from oodac runs `chs-smoke-ok`.

**Not claimed:** full SPEC product self-host, LSP/pkg, full LLVM CHS product.

`DESIGN.md` unchanged.

## Milestones

| M | Deliverable |
|---|---|
| M0 | Host surface (prior): FS, List, string walk, struct, argv |
| M1 | CHS freeze + `ooda dump` + oodac lexer parity |
| M2 | oodac AST dump (PROGRAM + FN scan) vs stage-0 dump presence |
| M3 | oodac check accept/reject corpus parity |
| M4 | CHS→C backend + gcc native stage-1 oodac |
| M5 | `scripts/fixed_point.sh` referee |

## How to verify

```bash
cargo test
./scripts/chs_parity.sh
./scripts/fixed_point.sh   # requires gcc; uses $HOME/.cache/ooda-tmp
```

## Key paths

- `bootstrap/CHS.md` — frozen matrix
- `oodac/main.oo` — oodac source
- `runtime/chs_rt.c` — native runtime
- `src/codegen_c.rs` — C backend
- `src/dump.rs` — canonical dumps
- `scripts/chs_parity.sh`, `scripts/fixed_point.sh`
