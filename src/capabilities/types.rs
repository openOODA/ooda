// ===================================================================
// openOODA Capability Effect System (default-deny)
//
// Real I/O is only allowed through a sealed table of effectful builtins.
// Each entry requires a specific capability type on the enclosing function.
// Renaming calls cannot invent new I/O primitives outside this table.
// ===================================================================
use crate::ast::*;

/// Which capability token an effectful operation requires.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapKind {
    Net,
    Fs,
    Sys,
    Env,
}

impl CapKind {
    pub fn type_name(self) -> &'static str {
        match self {
            CapKind::Net => "&NetCap",
            CapKind::Fs => "&FsCap",
            CapKind::Sys => "&SysCap",
            CapKind::Env => "&EnvCap",
        }
    }

    pub fn matches_type(self, t: &Type) -> bool {
        match (self, t) {
            (CapKind::Net, Type::NetCap) => true,
            (CapKind::Fs, Type::FsCap) => true,
            (CapKind::Sys, Type::SysCap) => true,
            (CapKind::Env, Type::EnvCap) => true,
            _ => false,
        }
    }
}

/// Sealed effectful builtin: only these names may perform side-effecting I/O.
#[derive(Debug, Clone, Copy)]
pub struct EffectBuiltin {
    /// Canonical call name as it appears after parsing (methods use ".name").
    pub name: &'static str,
    pub requires: CapKind,
    /// When true, args[0] (method receiver) must be a capability parameter handle.
    /// When false, the enclosing function must declare the cap (ambient grant).
    pub receiver_is_cap: bool,
}

