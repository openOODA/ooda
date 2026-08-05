
struct Gen {
    structs: HashMap<String, Vec<(String, Type)>>,
    type_aliases: HashMap<String, Type>,
    /// name → C return type string
    fn_ret: HashMap<String, String>,
    functions: Vec<String>,
    prelude: String,
    body: String,
    tmp: usize,
    /// When true, bare `return;` becomes `return 0;` (C main).
    c_main: bool,
    /// Current OODA function returns void (bare return;).
    fn_void: bool,
    /// Emit host FFI decls (only when program calls chs_build/host_*).
    with_host_ffi: bool,
}

