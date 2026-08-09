# Release checklist (pin lock habit)

**Not a beta gate.** Owner tags beta only via `BETA.md`. This checklist is **alpha ship hygiene**.

Use every time you cut or re-cut a pin (e.g. `v0.182.1-alpha`).

---

## 1. Pin lock (single string)

| Artifact | Must match |
|----------|------------|
| `install/BOOTSTRAP_PIN` | `vX.Y.Z-alpha` (or owner-chosen prerelease) |
| `ooda version` output | same version string (no `v` prefix in CLI is OK if documented) |
| `RELEASE_NOTES_vX.Y.Z-alpha.md` | filename + title |
| Git tag (when published) | same as pin |
| GitHub Release asset | `ooda-<tag>-linux-x86_64.tar.gz` + `.sha256` |

Do **not** leave site/install docs claiming a different pin without an explicit residual note.

---

## 2. Release notes (template)

Create `RELEASE_NOTES_<tag>.md` at repo root (copy prior pin and edit):

```markdown
# vX.Y.Z-alpha

**Not beta.** Owner-gated beta criteria live in `bootstrap/BETA.md`.

## Highlights

- (what shipped this pin)

## Build / install

\`\`\`bash
export SEED_OODAC="\${SEED_OODAC:-./oodac/oodac}"
./scripts/bootstrap_no_cargo.sh
./bin/ooda version
\`\`\`

Requires: bash, gcc, trusted seed binary (pure product rebuild).

## Pin

\`install/BOOTSTRAP_PIN\` = \`vX.Y.Z-alpha\`

## Residual honesty

- Fail-closed / out of product: see \`bootstrap/P4_DROPS.md\`
- Seed: cold start needs prebuilt pure \`oodac\` once
```

---

## 3. Pack + verify (pure product)

```bash
# 1) Pure rebuild
export SEED_OODAC="${SEED_OODAC:-./oodac/oodac}"
./scripts/bootstrap_no_cargo.sh

# 2) Product rails (local)
./scripts/ci_product.sh
./scripts/check_file_lines.sh   # O=0 when claimed

# 3) Tarball
./scripts/release.sh            # reads install/BOOTSTRAP_PIN
# → dist/ooda-<tag>-linux-x86_64.tar.gz (+ .sha256)

# 4) Offline dress rehearsal
RELEASE_TARBALL=dist/ooda-<tag>-linux-x86_64.tar.gz \
  ./scripts/install_dress_rehearsal.sh
```

---

## 4. Publish (optional this pin)

- Upload tarball **and** `.sha256` to GitHub Releases for the pin tag.
- Remote CI (`.github/workflows/product.yml`) downloads that pin as seed when
  `bootstrap/seed/oodac` is absent — **keep pin + asset + sha256 coherent**.
- Do **not** force a beta tag. Alpha/rc is fine indefinitely.

---

## 5. Anti-checklist

- [ ] Product purity: no `Cargo.toml` / no product `.rs` (B0/B1)
- [ ] No secrets in workflows or release notes
- [ ] No `curl | sh` to unpinned hosts in CI
- [ ] Residual features stay fail-closed (`P4_DROPS.md`), not soft-pass
- [ ] `BETA.md` unchanged unless owner edits In/Out surface

---

*Habit: pin → notes → rails → release.sh → dress → (optional) GitHub Release.*
