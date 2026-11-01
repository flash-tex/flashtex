//! Read-only proposals from the shared index. Approval/application is separate.
use crate::{number, string};
use flashtex_project_index::{ProjectIndex, SearchRequest, SourceSpan};
use serde_json::{json, Value};

// Leave ample room for the outer helper envelope; its complete size is still
// checked by OutputBuffer. This bounds serialized proposals, not process RSS.
const MAX_PLAN_BYTES: usize = 8 * 1024 * 1024;

pub fn handle(index: &ProjectIndex, kind: &str, p: &Value) -> Result<Value, String> {
    let snapshot = index.snapshot();
    if p["source_versions"] != json!(snapshot.documents)
        || number(p, "membership_generation")? != snapshot.generation
    {
        return Err("source snapshot changed; refresh before planning".into());
    }
    let max_bytes = usize::try_from(number(p, "max_bytes")?)
        .ok()
        .filter(|n| (1..=MAX_PLAN_BYTES).contains(n))
        .ok_or("max_bytes must be 1..8388608")?;
    let encoded = if kind == "plan_literal_replacement" {
        let mut request = SearchRequest::literal(string(p, "literal")?);
        request.max_matches = usize::try_from(number(p, "max_matches")?)
            .ok()
            .filter(|n| (1..=1000).contains(n))
            .ok_or("max_matches must be 1..1000")?;
        request.max_work = usize::try_from(number(p, "max_work")?)
            .ok()
            .filter(|n| (1..=1_000_000).contains(n))
            .ok_or("max_work must be 1..1000000")?;
        request.documents =
            serde_json::from_value(p.get("documents").cloned().unwrap_or(Value::Null))
                .map_err(|e| e.to_string())?;
        let search = index
            .search_literal(&snapshot, &request, || false)
            .map_err(|e| e.to_string())?;
        let plan = index
            .plan_literal_replacement(&search, string(p, "replacement")?)
            .map_err(|e| e.to_string())?;
        index.serialize_literal_replacement_plan(&plan, max_bytes)
    } else {
        let plan = if kind == "plan_citation_rename_at" {
            let file = string(p, "path")?.to_owned();
            let source = SourceSpan {
                revision: *snapshot.documents.get(&file).ok_or("unknown source path")?,
                file,
                start_byte: usize::try_from(number(p, "start_byte")?)
                    .map_err(|_| "invalid offset")?,
                end_byte: usize::try_from(number(p, "end_byte")?).map_err(|_| "invalid offset")?,
            };
            index.plan_citation_rename_at(&snapshot, &source, string(p, "new_name")?)
        } else {
            index.plan_citation_rename(&snapshot, string(p, "old_name")?, string(p, "new_name")?)
        }
        .map_err(|e| e.to_string())?;
        index.serialize_citation_rename_plan(&plan, max_bytes)
    }
    .map_err(|e| e.to_string())?;
    let plan: Value = serde_json::from_str(&encoded).map_err(|e| e.to_string())?;
    Ok(
        json!({"source_versions":snapshot.documents,"membership_generation":snapshot.generation,"plan":plan}),
    )
}
