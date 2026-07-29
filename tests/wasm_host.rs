use anyhow::Result;
use wasmtime::*;

pub fn run_wat(wat: &str) -> Result<String> {
    let engine = Engine::default();
    let module = Module::new(&engine, wat)?;
    let mut store = Store::new(&engine, ());
    let mut linker = Linker::new(&engine);

    // Provide env.println
    linker.func_wrap("env", "println", |v: i64| {
        println!("{}", v);
    })?;

    // Provide env.println_str
    // We need memory access to read the string.
    // In wasmtime, we can capture the memory via caller if we use `Func::wrap`.
    linker.func_wrap("env", "println_str", |mut caller: Caller<'_, ()>, offset: i32| {
        let mem = caller.get_export("memory").unwrap().into_memory().unwrap();
        let data = mem.data(&caller);
        // String is NUL terminated
        let start = offset as usize;
        let mut end = start;
        while end < data.len() && data[end] != 0 {
            end += 1;
        }
        let s = std::str::from_utf8(&data[start..end]).unwrap();
        println!("{}", s);
    })?;

    // Provide env.streq
    linker.func_wrap("env", "streq", |mut caller: Caller<'_, ()>, a: i32, b: i32| -> i32 {
        let mem = caller.get_export("memory").unwrap().into_memory().unwrap();
        let data = mem.data(&caller);
        let start_a = a as usize;
        let mut end_a = start_a;
        while end_a < data.len() && data[end_a] != 0 { end_a += 1; }
        
        let start_b = b as usize;
        let mut end_b = start_b;
        while end_b < data.len() && data[end_b] != 0 { end_b += 1; }
        
        let sa = &data[start_a..end_a];
        let sb = &data[start_b..end_b];
        if sa == sb { 1 } else { 0 }
    })?;

    let instance = linker.instantiate(&mut store, &module)?;
    
    // Trap stdout to capture output if we wanted to? No, just run it for now.
    if let Some(main) = instance.get_typed_func::<(), i32>(&mut store, "main").ok() {
        main.call(&mut store, ())?;
    } else {
        anyhow::bail!("no main function");
    }
    
    Ok("".into())
}

#[test]
fn test_streq_println_str() {
    let wat = r#"
    (module
      (import "env" "println" (func $println (param i64)))
      (import "env" "println_str" (func $println_str (param i32)))
      (import "env" "streq" (func $streq (param i32 i32) (result i32)))
      (memory 1)
      (export "memory" (memory 0))
      (data (i32.const 1024) "hello\00")
      (data (i32.const 1030) "world\00")
      (data (i32.const 1036) "hello\00")
      (func (export "main") (result i32)
        (call $println_str (i32.const 1024))
        (call $println (i64.extend_i32_s (call $streq (i32.const 1024) (i32.const 1036))))
        (call $println (i64.extend_i32_s (call $streq (i32.const 1024) (i32.const 1030))))
        (i32.const 0)
      )
    )
    "#;
    run_wat(wat).unwrap();
}
