# Bootstrap seed (optional local)

Place an executable pure `oodac` here as `bootstrap/seed/oodac` for offline CI
and cold machines that lack `oodac/oodac` in the working tree.

## Resolution order (CI / bootstrap)

1. `SEED_OODAC` environment variable  
2. `bootstrap/seed/oodac` (this directory)  
3. `oodac/oodac` or `oodac/oodac2` in the checkout (local builds; gitignored)  
4. GitHub Release tarball for `install/BOOTSTRAP_PIN` (see `.github/workflows/no_rust.yml`)

## How to populate

```bash
# From a tree that already bootstrapped:
cp -a oodac/oodac bootstrap/seed/oodac
chmod +x bootstrap/seed/oodac

# Or extract from a release pack:
tar -xzf dist/ooda-v*-linux-x86_64.tar.gz
cp -a ooda-v*-linux-x86_64/oodac/oodac bootstrap/seed/oodac
```

Binary may be gitignored org-wide; committing it is optional. Remote CI uses the
pinned release asset + `.sha256` when this file is absent.

**Never** use a Cargo-built host as seed. Seed must be pure `.oo`+C product.
