# TOOLS.md — Process Lenses for openOODA Rotations

**openOODA Project** — colocated with [`DESIGN.md`](./DESIGN.md)

This file is a **process layer**, not the product north star.  
`DESIGN.md` = what OODA is and must remain.  
`TOOLS.md` = how an agent **picks and applies** decision math during Observe → Decide → Act → Ship → Propagate → Validate → Lock.

---

## How to use this menu

1. **Read `DESIGN.md` first** when the task touches language shape, security model, dual-engine intent, or AI surface. Never invent architecture mid-rotation.
2. **At Decide**, name **one primary tool** (and optionally one secondary) from the menu below. State why that lens fits *this* top-5 set.
3. **Do not invent new equations mid-rotation.** If a better lens appears, note it in PROGRESS / ship notes for a later TOOLS edit — then finish the rotation with a listed tool.
4. **Do not edit `DESIGN.md`** (or core-spec grammar/spec) from rotation work. Process changes land here; product truth stays in DESIGN / SPEC.
5. Tools rank **work**, not pillars by brand. Zero-Rust / R1 self-host is a peer pillar in ranking, but leverage is always D↓ / W↓ / honesty / path-to-beta.

---

## Menu (pick at Decide)

### 1. Energy–Maneuverability (E-M) — primary default

**Equation:**  
\[
P_s = V \cdot \frac{T - D}{W}
\]

| Symbol | Meaning in openOODA work |
|--------|---------------------------|
| \(P_s\) | Specific excess power — how much “climb” the change buys |
| \(V\)   | Velocity / tempo — feedback speed, ship cadence, time-to-signal |
| \(T\)   | Thrust — real capability delivered (tests green, surface honest, pin true) |
| \(D\)   | Drag — architectural friction, dual paths, silent-OK lies, ceremony |
| \(W\)   | Weight — heap, duplicate logic, stage-0 surface area, fake complexity |

**Use when:** almost every rotation — prioritize slices that **raise \(T\)**, **cut \(D\)**, or **cut \(W\)** without killing \(V\).  
**Reject when:** the change adds theater (docs-only, pin theater, soft fails) that inflates \(T\) on paper only.

**Act constraints:** zero-cost abstractions where practical; prefer stack/local over heap; gate unused host/imports; fail-closed unfinished surface.

---

### 2. Power law (Pareto Decide)

**Idea:** A small fraction of issues dominate outcome. Rank the **top 5** only; ignore long tails until the head moves.

**Use when:** Observe produced a large backlog; need a ruthless top-5.  
**Act:** implement the ranked five (or Circuit Breaker after 2 fails → restore, log, pivot). Do not “polish the long tail” instead of the head.

---

### 3. First principles

**Idea:** Strip to invariants from DESIGN + physical constraints of the compiler (parse → check → lower → eng → ship). Ask: *what must be true for this to be OODA?*

**Use when:** debate is fashion, framework habit, or “other langs do X”; when stage-0 vs oodac diverge and the right owner is unclear.  
**Reject:** solutions that violate sealed caps, dual-engine honesty, fail-closed unfinished work, or pin-triple integrity.

---

### 4. Honesty budget

**Idea:** Every silent-OK, soft pass, or “looks green but lies” spends trust. Budget is near zero for shipped surface.

**Equation (qualitative):**  
\[
\text{Trust} \propto \frac{\text{true signals}}{\text{false greens} + \text{untested claims}}
\]

**Use when:** diagnostics, typecheck plateaus, CHS parity gaps, WASM/host mismatches, “works in one engine only.”  
**Act:** prefer **fail-closed** or **explicit error** over silent accept; golden tests that catch the lie; no theater locks.

---

### 5. Amdahl’s law (bottleneck share)

**Equation:**  
\[
S_{\text{overall}} = \frac{1}{(1 - P) + \frac{P}{S}}
\]

| Symbol | Meaning |
|--------|---------|
| \(P\) | Fraction of the path that the improvement actually touches |
| \(S\) | Speedup (or quality uplift) on that fraction only |

**Use when:** optimizing hot paths, token-scan TC vs full env, codegen vs check, “one more micro-opt.”  
**Reject:** large \(S\) on tiny \(P\) sold as product progress (e.g. polish a rare fixture while silent-OK still owns the main path).

---

### 6. Little’s law (flow / WIP)

**Equation:**  
\[
L = \lambda W
\]

| Symbol | Meaning |
|--------|---------|
| \(L\) | Work-in-progress (open branches, half-ships, dirty trees, untagged pins) |
| \(\lambda\) | Throughput (honest ships / unit time) |
| \(W\) | Latency of an item in the system (observe → lock) |

**Use when:** multi-loop campaigns, dirty locks, partial pin bumps, stacked unfinished slices.  
**Act:** finish or restore; **lock-before-tag**; forward-only pin triple; serial N+1 only after N locks clean.

---

### 7. Assembly depth to B0 (beta distance)

**Idea:** Beta B0–B5 require **zero `.rs` in the product path**. Measure work by whether it **shortens assembly depth** toward R1/oodac ownership of the surface, or **deepens** stage-0 Rust.

**Use when:** ranking zero-Rust vs stage-0 convenience; choosing where new contracts/fixtures live (`.oo` first).  
**Act:** new surface and fixtures in `.oo`; stage-0 only as temporary host; no new Rust product surface without an explicit oodac path plan.

---

### 8. Blue Ocean (strategy only)

**Idea:** Compete where DESIGN already differentiates — sealed caps, contracts, AI-native diagnostics, dual engine — not by cloning commodity language checklists.

**Use when:** roadmap / top-5 *selection* only (what class of problem to attack).  
**Do not use** as an excuse to skip honesty, tests, or pin discipline. Strategy picks the hill; E-M / honesty / Little run the assault.

---

## Rotation checklist (tool-aware)

| Phase | Tool role |
|-------|-----------|
| Observe | Collect facts; no equation shopping yet |
| Decide | Name primary (+ optional secondary) tool; emit top-5 ranked by that lens |
| Act | Apply E-M constraints + honesty; Circuit Breaker = 2 fails → restore + PROGRESS |
| Ship | Real change, real tests, LLM sign-off; version forward only |
| Propagate | Truthful public copy; no overclaim |
| Validate / Lock | Clean git + green suite; lock-before-tag; no theater |

---

## Hard rules (non-tools, always on)

- **No `DESIGN.md` / core-spec edits** from rotation process work.
- **Fail-closed** unfinished features.
- **Pin triple** aligned (git tag, GitHub Release, website install pin); forward-only.
- **Stage-0 Rust isolated**; new contracts/fixtures in `.oo`.
- **No inventing tools mid-rotation** — extend this file in a dedicated doc change if needed.
- **LLM name/version sign-off** on ships when the session protocol requires it.

---

## Quick pick guide

| Situation | Prefer |
|-----------|--------|
| Default ranking / implement slice | **E-M** (+ Power law for top-5) |
| Silent-OK / dual-engine lie | **Honesty budget** then E-M |
| Huge backlog | **Power law** |
| “Should this even be Rust?” | **Assembly depth to B0** + first principles |
| WIP / dirty locks / multi-loop | **Little’s law** |
| Micro-opt vs product path | **Amdahl** |
| What class of problem is worth it? | **Blue Ocean** (Decide only) |
| Fashion vs invariant | **First principles** + DESIGN |

---

*Process document. Product truth: [`DESIGN.md`](./DESIGN.md).*
