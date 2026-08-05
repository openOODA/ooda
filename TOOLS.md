# TOOLS.md — Science-informed lenses (build fast, spend few tokens)

**Product:** [`DESIGN.md`](./DESIGN.md) · **Beta:** [`bootstrap/BETA.md`](bootstrap/BETA.md)  
**Protocol:** home `loop - openOODA.md` · **Handoff:** monorepo `PROGRESS.md`

Rotations **build openOODA quickly** and **save tokens**. Real-world science
is the teacher. Tools rank *work* — they do not edit language architecture (DESIGN).

---

## Always on (not chosen)

| Rule | Science cousin | Meaning |
|------|----------------|---------|
| **E-M** | Physics / flight | Rank + Act every turn. List in PROGRESS. |
| **Entropy \(S\)** | Thermodynamics | Score disorder; good ships **lower \(S\)** (below). |
| **≤250 lines** | Materials / modularity | Owned source file hard cap (SPEC intent). See split plan. |
| **Power Law** | Math / Pareto | Top **≤5**. Ignore the long tail this turn. |
| **DESIGN** | — | Architecture from DESIGN only. |
| **Tests+code** | Biology / immune | Pass+fail fixtures ship with behavior. |
| **No hand-waves** | Chemistry / purity | Unfinished → fail-closed. Stuck → Blockers. |
| **`.oo` product** | — | No new Rust-only surface. Thin C glue OK. |
| **Feedback** | Control / OODA | Shorten observe→signal (faster true fail/pass). |

---

## Entropy \(S\) — thermodynamics (always measure)

Disorder in the **trust surface** (not lines of code). Untested claims and lies
raise entropy; quality tests and fail-closed truth **export** disorder.

### Score (integer, recompute each Observe / Ship)

\[
S = U + D + F + W + O
\]

| Term | Count of (each item = 1 unless noted) |
|------|----------------------------------------|
| **\(U\)** | **Untested claims** — behaviors asserted with **no** committed pass+fail rail |
| **\(D\)** | **Dual-engine disagreements** — same fixture, engines differ on OK/ERR when they should agree |
| **\(F\)** | **Fail-open holes** — silent-OK / soft-pass / missing fail fixture on critical paths |
| **\(W\)** | **Hand-waves** — stubs that present as done without fail-closed residual |
| **\(O\)** | **Oversize files** — owned source files with **> 250 lines** (`scripts/check_file_lines.sh`) |

**File-size cap (always):** Owned `.oo` / `.rs` / hand `.c`/`.h` / `.sh` ≤ **250 lines**.  
Exclude generated emit (`*.oo.c`, `oodac/main.c`, `oodac/oodac2.c`, `target/`, `dist/`).  
Plan: [`bootstrap/SPLIT_PLAN.md`](bootstrap/SPLIT_PLAN.md).  
Lock: `./scripts/check_file_lines.sh` (strict when aiming O=0; `--ratchet` while splitting — fail if oversize **grows** or new oversize appears).

**Rules of counting (anti-theater):**

- Prefer **conservative** counts: if unsure a claim is tested, it is \(U\).
- **\(O\)** comes from the checker, not vibes. Growing an oversize file raises \(O\) weight and fails ratchet.
- **\(S \downarrow\)** = improvement worth claiming. **\(S \uparrow\)** or **\(O \uparrow\)** = bad ship unless explained.
- **\(S\) flat** OK only with one-line why. Proxy, not lab thermo.
- **Not `RS_COUNT`** (host `.rs` count). Report `RS_COUNT`, \(S\), and \(O\) while residual host exists.

### Report in PROGRESS every pin

```text
S: <n> (Δ … ↓|↑|flat) — U=_ D=_ F=_ W=_ O=_
O: <n> (Δ …)  # same O as in S; list top oversize paths if O>0
```

Example: `S: 36 (Δ -2 ↓) — U=4 D=2 F=5 W=1 O=24`

**Worth it?** Feature that raises \(O\) or \(S\) fails the entropy test. Splits that drop \(O\) are first-class progress.

---

## E-M — physics (always)

\[
P_s = V \cdot \frac{T - D_{\mathrm{drag}}}{W_{\mathrm{mass}}}
\]

| Symbol | Science | openOODA |
|--------|---------|----------|
| \(V\) | Speed | Tempo — small ships, fast fail |
| \(T\) | Thrust | Capability **locked by quality tests** |
| \(D_{\mathrm{drag}}\) | Drag | Lies, ceremony — same spirit as high \(S\) |
| \(W_{\mathrm{mass}}\) | Weight | Duplicate logic, residual host bulk |

**Raise \(T\), cut drag/weight, protect \(V\), drive \(S \downarrow\) and \(O \downarrow\).**  
Monofile growth is \(W_{\mathrm{mass}}\) and \(O\) — reject it.

**Chemistry under E-M (not a separate pick):**

- **Activation energy** — low barrier to a *true* green test  
- **Catalyst** — fixtures/harnesses that cheapen many later reactions  
- **Purify before grow** — drop \(S\)/\(O\) (split or kill lie) before stacking features  

---

## Optional second tool (0–1 with E-M — list if used)

### Honesty budget + immune rail — biology / trust

\[
\text{Trust} \propto \frac{\text{true signals}}{\text{false greens} + \text{untested claims}}
\]

Directly attacks \(U\), \(D\), \(F\), \(W\) (components of \(S\)).

| Science | Act |
|---------|-----|
| **Immune / negative selection** | Fail fixtures reject non-self |
| **Error budget ≈ 0** | No accumulating false greens |
| **Entropy export** | Each true fail+pass fixture should lower \(S\) |

**Default second** when hunting lies. Never install non-truth to look done.

---

### Information value of the next probe — information theory

Prefer the **smallest probe** that cuts uncertainty most (often largest expected \(\Delta S \downarrow\) per token).

**Use when:** audit; many claims; “what to test first?”  
**Act:** one falsifying fixture/command; then code from that signal.

---

### Assembly depth — systems / materials

Residual host = substrate debt. Boundary defects raise long-run \(S\).

**Use when:** host vs `.oo`/C path. No hollow self-host.

---

### Little’s law — math / flow

\[
L = \lambda W
\]

**Use when:** WIP / dirty locks. Finish or restore; protect \(V\).

---

### Amdahl — math / bottlenecks

\[
S_{\text{overall}} = \frac{1}{(1-P)+\frac{P}{S}}
\]

**Use when:** measured hot path. Don’t polish tiny \(P\) while \(S\) (entropy) stays high.

---

### Homeostasis — biology / ecology (rare)

**Use when:** overload. Fewer true ships; cull invasives; invest in immune fixtures (\(S \downarrow\)).

---

## Sciences map

| Domain | Steal | Where |
|--------|-------|-------|
| **Thermodynamics** | Entropy \(S\), export disorder via tests | Always measure |
| **Physics** | E-M | Always Act |
| **Math** | Power law, Little, Amdahl, info value | Rules + optional |
| **Chemistry** | Activation, catalyst, purify | Under E-M |
| **Biology** | Immune, homeostasis | Honesty; rare tool |
| **Information theory** | Value of next probe | Optional second |
| **Control** | Short feedback | Always-on |
| **Materials / ecology** | Boundaries, invasives | Assembly; homeostasis |

**Do not use as Act tools:** Blue Ocean / marketing slogans.

---

## Decide → Act → Lock

1. **Observe:** recompute \(S\); run `scripts/check_file_lines.sh` → \(O\); `RS_COUNT` if residual `.rs`.  
2. **Decide:** ≤5 → E-M → optional second tool; prefer \(S \downarrow\) / \(O \downarrow\) (splits count).  
3. **Act:** code + tests; do not grow oversize files; 2 fails → restore + Blockers → pivot.  
4. **Lock / Ship:** line checker (ratchet or strict); suite green; report \(S\), \(O\), \(\Delta\).

### Quick pick (second tool)

| Situation | Second |
|-----------|--------|
| Default | *(none)* |
| Lies / silent-OK / lower \(S\) | **Honesty + immune** |
| What to measure first | **Information value** |
| Host vs `.oo` | **Assembly depth** |
| WIP clog | **Little** |
| Hot path | **Amdahl** |
| Overload | **Homeostasis** |

---

*Process only. \(S\) is a proxy number for trust-disorder — use it; don’t worship false precision.*  
*Science teaches; DESIGN governs product; tests export entropy.*
