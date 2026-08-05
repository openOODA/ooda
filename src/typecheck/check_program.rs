impl TypeChecker {

    pub fn check_program(program: &Program) -> Result<()> {
        let mut tc = TypeChecker {
            functions: HashMap::new(),
            param_refinements: HashMap::new(),
            type_aliases: HashMap::new(),
            alias_refinements: HashMap::new(),
            active_list_lens: std::cell::RefCell::new(HashMap::new()),
            current_return: std::cell::RefCell::new(None),
            loop_depth: std::cell::Cell::new(0),
        };

        // Collect type aliases first (named structs for StructLit).
        for item in &program.items {
            if let Item::TypeAlias(name, ty) = item {
                if let Some(b) = int_refinement_bounds(ty) {
                    tc.alias_refinements.insert(name.clone(), b);
                }
                tc.type_aliases
                    .insert(name.clone(), Ty::from_ast(ty));
            }
        }

        // Builtins
        tc.functions
            .insert("println".into(), (vec![Ty::Unknown], Ty::Void));
        tc.functions
            .insert("assert_eq".into(), (vec![Ty::Unknown, Ty::Unknown], Ty::Void));
        tc.functions
            .insert("assert_is_err".into(), (vec![Ty::Unknown], Ty::Void));
        // CHS list surface
        tc.functions
            .insert("list_new".into(), (vec![], Ty::List(Box::new(Ty::Unknown))));
        tc.functions.insert(
            "list_push".into(),
            (
                vec![Ty::List(Box::new(Ty::Unknown)), Ty::Unknown],
                Ty::List(Box::new(Ty::Unknown)),
            ),
        );
        tc.functions.insert(
            "list_get".into(),
            (
                vec![Ty::List(Box::new(Ty::Unknown)), Ty::Int],
                Ty::Unknown,
            ),
        );
        tc.functions.insert(
            "list_len".into(),
            (vec![Ty::List(Box::new(Ty::Unknown))], Ty::Int),
        );
        // CHS string walk
        tc.functions
            .insert("chars_len".into(), (vec![Ty::String], Ty::Int));
        tc.functions
            .insert("char_at".into(), (vec![Ty::String, Ty::Int], Ty::String));
        tc.functions.insert(
            "str_slice".into(),
            (vec![Ty::String, Ty::Int, Ty::Int], Ty::String),
        );
        tc.functions
            .insert("char_is_digit".into(), (vec![Ty::String], Ty::Bool));
        tc.functions
            .insert("char_is_alpha".into(), (vec![Ty::String], Ty::Bool));
        tc.functions
            .insert("char_is_space".into(), (vec![Ty::String], Ty::Bool));
        // Host bootstrap APIs (exact stage-0 dumps + real CHS native build)
        tc.functions
            .insert("host_ast_dump".into(), (vec![Ty::String], Ty::String));
        tc.functions
            .insert("host_check".into(), (vec![Ty::String], Ty::String));
        tc.functions
            .insert("host_token_dump".into(), (vec![Ty::String], Ty::String));
        tc.functions.insert(
            "chs_build".into(),
            (
                vec![Ty::String, Ty::String],
                Ty::Result(Box::new(Ty::String), Box::new(Ty::String)),
            ),
        );
        tc.functions
            .insert("process_exit".into(), (vec![Ty::Int], Ty::Void));
        // Host helpers: object-cap shape (live handle first, then op args).
        let res_v_host = Ty::Result(Box::new(Ty::Void), Box::new(Ty::String));
        let res_s_host = Ty::Result(Box::new(Ty::String), Box::new(Ty::String));
        for n in ["mkdir_p", "chmod_exec"] {
            // (cap, path)
            tc.functions
                .insert(n.into(), (vec![Ty::Unknown, Ty::Unknown], res_v_host.clone()));
        }
        for n in ["copy_file", "http_download", "extract_tar_gz"] {
            // (cap, a, b)
            tc.functions.insert(
                n.into(),
                (
                    vec![Ty::Unknown, Ty::Unknown, Ty::Unknown],
                    res_v_host.clone(),
                ),
            );
        }
        // path_exists(cap, path) — Bool, not Result
        tc.functions.insert(
            "path_exists".into(),
            (vec![Ty::Unknown, Ty::Unknown], Ty::Bool),
        );
        // sys_exec is varargs at runtime; registered for lookup, arity special-cased below.
        tc.functions.insert(
            "sys_exec".into(),
            (
                vec![Ty::Unknown, Ty::Unknown],
                res_s_host.clone(),
            ),
        );
        tc.functions.insert(
            "exec".into(),
            (
                vec![Ty::Unknown, Ty::Unknown],
                res_s_host.clone(),
            ),
        );
        tc.functions.insert(
            "crypto_sha256_internal".into(),
            (vec![Ty::String], Ty::String),
        );
        tc.functions.insert(
            "crypto_hmac_sha256_internal".into(),
            (vec![Ty::String, Ty::String], Ty::String),
        );
        tc.functions.insert(
            "json_parse_internal".into(),
            (
                vec![Ty::String],
                Ty::Result(Box::new(Ty::String), Box::new(Ty::String)),
            ),
        );
        tc.functions.insert(
            "json_stringify_internal".into(),
            (vec![Ty::Unknown], Ty::String),
        );
        tc.functions.insert(
            "async_spawn_internal".into(),
            (vec![Ty::Unknown, Ty::Unknown], Ty::String),
        );
        tc.functions.insert(
            "async_join_internal".into(),
            (
                vec![Ty::Unknown, Ty::Unknown],
                Ty::Result(Box::new(Ty::String), Box::new(Ty::String)),
            ),
        );
        tc.functions.insert(
            "python_embed_internal".into(),
            (
                vec![Ty::Unknown, Ty::Unknown],
                Ty::Result(Box::new(Ty::String), Box::new(Ty::String)),
            ),
        );
        // Ok/Err construct Result — typed as Result[T, _] / Result[_, E] loosely
        tc.functions.insert(
            "Ok".into(),
            (
                vec![Ty::Unknown],
                Ty::Result(Box::new(Ty::Unknown), Box::new(Ty::Unknown)),
            ),
        );
        tc.functions.insert(
            "Err".into(),
            (
                vec![Ty::Unknown],
                Ty::Result(Box::new(Ty::Unknown), Box::new(Ty::Unknown)),
            ),
        );
        tc.functions.insert(
            "Some".into(),
            (
                vec![Ty::Unknown],
                Ty::Option(Box::new(Ty::Unknown)),
            ),
        );
        tc.functions.insert(
            "None".into(),
            (vec![], Ty::Option(Box::new(Ty::Unknown))),
        );
        // Sealed effects (object-cap): first formal is the live handle; remaining
        // are operation args. Arity is enforced for user fns; these use concrete
        // counts so wrong call shape fails closed (no soft 1-arg theater).
        let res_s = Ty::Result(Box::new(Ty::String), Box::new(Ty::String));
        let res_v = Ty::Result(Box::new(Ty::Void), Box::new(Ty::String));
        for name in ["fetch", "downloadData", "http_get", "net_get", "net_connect"] {
            // (cap, url) or ambient-shaped still checked at cap pass
            tc.functions
                .insert(name.into(), (vec![Ty::Unknown, Ty::Unknown], res_s.clone()));
        }
        for name in ["read_file", "fs_read"] {
            tc.functions
                .insert(name.into(), (vec![Ty::Unknown, Ty::Unknown], res_s.clone()));
        }
        for name in ["env_get"] {
            tc.functions
                .insert(name.into(), (vec![Ty::Unknown, Ty::Unknown], res_s.clone()));
        }
        for name in ["write_file", "fs_write"] {
            // (cap, path, content)
            tc.functions.insert(
                name.into(),
                (vec![Ty::Unknown, Ty::Unknown, Ty::Unknown], res_v.clone()),
            );
        }
        for name in ["env_set"] {
            tc.functions.insert(
                name.into(),
                (vec![Ty::Unknown, Ty::Unknown, Ty::Unknown], res_v.clone()),
            );
        }
        for name in ["sys_exec", "exec", "spawn_process"] {
            // (cap, cmd) typical object-cap shape
            tc.functions
                .insert(name.into(), (vec![Ty::Unknown, Ty::Unknown], res_s.clone()));
        }

        for item in &program.items {
            if let Item::Function(f) = item {
                let params: Vec<Ty> = f.params.iter().map(|p| Ty::from_ast(&p.param_type)).collect();
                let ret = Ty::from_ast(&f.return_type);
                let bounds: Vec<Option<(i64, i64)>> = f
                    .params
                    .iter()
                    .map(|p| {
                        int_refinement_bounds(&p.param_type).or_else(|| {
                            if let Type::Custom(name) = &p.param_type {
                                tc.alias_refinements.get(name).copied()
                            } else {
                                None
                            }
                        })
                    })
                    .collect();
                tc.functions.insert(f.name.clone(), (params, ret));
                tc.param_refinements.insert(f.name.clone(), bounds);
            }
        }
        for item in &program.items {
            if let Item::Function(f) = item {
                tc.check_function(f)?;
            }
        }
        Ok(())
    }
}
