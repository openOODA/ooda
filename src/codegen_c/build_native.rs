impl CCodeGen {
    pub fn emit_c(program: &Program) -> Result<String> {
        Self::assert_chs_c_subset(program)?;
        let mut g = Gen::new();
        g.with_host_ffi = program_needs_host_ffi(program);
        g.emit_program(program)?;
        Ok(g.finish())
    }

    fn assert_chs_c_subset(program: &Program) -> Result<()> {
        let aliases = program.collect_type_aliases();
        for item in &program.items {
            if let Item::Function(f) = item {
                for p in &f.params {
                    Self::check_ty(&p.param_type.resolve_alias(&aliases), &f.name)?;
                }
                Self::check_ty(&f.return_type.resolve_alias(&aliases), &f.name)?;
            }
        }
        Ok(())
    }

    fn check_ty(t: &Type, ctx: &str) -> Result<()> {
        match t {
            Type::Int | Type::Bool | Type::Void | Type::String | Type::Float => Ok(()),
            Type::FsCap | Type::EnvCap | Type::SysCap | Type::NetCap => Ok(()),
            Type::List(inner) => match **inner {
                Type::Int | Type::String => Ok(()),
                _ => bail!("C backend List only supports List[Int]|List[String] in '{}'", ctx),
            },
            Type::Struct { .. } => Ok(()),
            Type::Option(_) | Type::Result(_, _) => Ok(()),
            Type::Custom(s) => match s.as_str() {
                "Int" | "Bool" | "String" | "Void" | "Float" => Ok(()),
                _ => Ok(()), // named struct aliases
            },
        }
    }

    /// Compile .oo → native binary via gcc + chs_rt.c. Returns path to binary.
    ///
    /// **Assembly depth:** pure CHS programs (no `chs_build` / host dumps) link
    /// with **only** gcc + `chs_rt.c` — no `libooda.a` / Cargo staticlib.
    /// Host-FFI programs still require stage-0 `libooda.a` (fail closed if missing).
    pub fn build_native(program: &Program, out_bin: &Path, rt_c: &Path, release: bool) -> Result<()> {
        let need_host = program_needs_host_ffi(program);
        let c_src = Self::emit_c(program)?;
        let out_c = out_bin.with_extension("c");
        std::fs::write(&out_c, &c_src)?;
        let gcc = which_gcc()?;
        // Prefer HOME cache for compiler temp files — /tmp may be quota-limited.
        let tmp = std::env::var("TMPDIR").unwrap_or_else(|_| {
            let p = dirs_tmp();
            let _ = std::fs::create_dir_all(&p);
            p
        });
        let mut cmd = Command::new(&gcc);
        let opt_flag = if release { "-O3" } else { "-O0" };
        cmd.env("TMPDIR", &tmp)
            .env("TMP", &tmp)
            .env("TEMP", &tmp)
            .arg(opt_flag);

        if release {
            cmd.arg("-flto");
        }

        cmd.arg("-std=c99");
        if need_host {
            // Enable host FFI wrappers in chs_rt.c; link stage-0 staticlib.
            cmd.arg("-DOODA_WITH_HOST_FFI");
            let lib_dir = find_ooda_staticlib_dir().ok_or_else(|| {
                anyhow::anyhow!(
                    "CHS C backend: program uses host FFI (chs_build/host_* dumps) but \
                     libooda.a not found under target/{{release,debug}}. \
                     Run `cargo build --release` or use pure CHS without host builtins."
                )
            })?;
            cmd.arg(&out_c).arg(rt_c);
            cmd.arg(format!("-L{}", lib_dir.display()));
            cmd.arg("-looda");
            cmd.arg("-lpthread");
            cmd.arg("-ldl");
            cmd.arg("-lm");
            // Rust staticlib may need libgcc_s / libc
            cmd.arg("-Wl,--allow-multiple-definition");
        } else {
            // Pure CHS: gcc + runtime only — no Cargo/staticlib (B0 assembly depth).
            cmd.arg(&out_c).arg(rt_c);
        }
        cmd.arg("-o").arg(out_bin);
        let out = cmd
            .output()
            .map_err(|e| anyhow::anyhow!("failed to spawn {}: {}", gcc, e))?;
        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr);
            bail!("gcc failed linking CHS C backend:\n{}", err.chars().take(1200).collect::<String>());
        }
        Ok(())
    }
}
