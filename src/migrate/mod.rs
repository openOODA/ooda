// job: edition migrator (match wildcards + let-mut)
// in:  .oo source path + edition
// out: rewritten source / MigrateReport
// stage: host
include!("engine.rs");
include!("let_mut.rs");
include!("match_wild.rs");
include!("tests.rs");
