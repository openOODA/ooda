// ===================================================================
// openOODA LSP — textDocumentSync diagnostics + WorkspaceEdit codeActions
// ===================================================================
//
// Honest surface (not a full language server):
//   - textDocumentSync Incremental (kind 2): didOpen + ranged didChange
//     (full-document changes without `range` still accepted)
//   - textDocument/codeAction: WorkspaceEdit for let→let mut, missing return,
//     return-type mismatch, undefined var stub, missing `: Type` on params
//   - No completion / hover / rename
//
// Document texts live in a process-local HashMap (uri → source). codeAction
// consults that store — never re-reads disk (editor buffer is source of truth).
// ===================================================================
use anyhow::Result;
use crate::diagnostics::{byte_offset_to_lsp, lsp_position_to_byte_offset, parse_loc, to_lsp_position};
use std::collections::HashMap;
use std::io::{BufRead, Read, Write};

pub struct LspDaemon;

/// Apply LSP `contentChanges` (incremental ranges and/or full replaces) to `text`.
/// Pure: no I/O. Positions are clamped via `lsp_position_to_byte_offset`.
/// Inverted ranges are ordered so `replace_range` never panics.
pub fn apply_content_changes(text: &str, changes: &[serde_json::Value]) -> String {
    let mut text = text.to_string();
    for change in changes {
        if let Some(range) = change.get("range") {
            let start = match range.get("start") {
                Some(s) => s,
                None => continue,
            };
            let end = match range.get("end") {
                Some(e) => e,
                None => continue,
            };
            let sl = start.get("line").and_then(|l| l.as_u64()).unwrap_or(0) as usize;
            let sc = start.get("character").and_then(|c| c.as_u64()).unwrap_or(0) as usize;
            let el = end.get("line").and_then(|l| l.as_u64()).unwrap_or(0) as usize;
            let ec = end.get("character").and_then(|c| c.as_u64()).unwrap_or(0) as usize;
            let new_text = change.get("text").and_then(|t| t.as_str()).unwrap_or("");
            let start_idx = lsp_position_to_byte_offset(&text, sl, sc);
            let end_idx = lsp_position_to_byte_offset(&text, el, ec);
            let (lo, hi) = if start_idx <= end_idx {
                (start_idx, end_idx)
            } else {
                (end_idx, start_idx)
            };
            text.replace_range(lo..hi, new_text);
        } else {
            // Full document replace (clients may still send this under Incremental).
            text = change
                .get("text")
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string();
        }
    }
    text
}

