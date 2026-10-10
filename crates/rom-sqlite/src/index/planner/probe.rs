//! Capped covering-index counts and conservative physical-plan recognition.
use super::{MAX_PROBE_KEYS, predicate::Predicate};
use crate::query_observation::ProbeMetrics;
use rusqlite::{Connection, params_from_iter, types::Value};

#[derive(Clone, Copy)]
pub(super) enum Count {
    Exact(u64),
    Saturated,
}

pub(super) fn count<const OBSERVED: bool>(
    c: &Connection,
    predicate: &Predicate,
    cap: u64,
    metrics: &mut ProbeMetrics,
) -> Option<Count> {
    let limit = cap.checked_add(1)?;
    if usize::try_from(limit).ok()? > MAX_PROBE_KEYS.checked_sub(metrics.rows)? {
        return None;
    }
    let sql = predicate.probe_sql();
    let mut parameters = predicate.parameters.clone();
    parameters.push(Value::Integer(i64::try_from(limit).ok()?));
    if !recognized(c, &sql, &parameters, false)? {
        return None;
    }
    let mut statement = c.prepare(&sql).ok()?;
    #[cfg(feature = "test-support")]
    let start = OBSERVED.then(std::time::Instant::now);
    metrics.statements += 1;
    let counted = (|| {
        let mut cursor = statement.query(params_from_iter(&parameters)).ok()?;
        let mut rows = 0u64;
        while cursor.next().ok()?.is_some() {
            rows = rows.checked_add(1)?;
            metrics.rows = metrics.rows.checked_add(1)?;
            if rows > limit || metrics.rows > MAX_PROBE_KEYS {
                return None;
            }
        }
        Some(if rows <= cap {
            Count::Exact(rows)
        } else {
            Count::Saturated
        })
    })();
    #[cfg(feature = "test-support")]
    if let Some(start) = start {
        metrics.elapsed_ns = metrics.elapsed_ns.checked_add(start.elapsed().as_nanos())?;
        metrics.vm_steps = metrics.vm_steps.checked_add(
            u64::try_from(statement.get_status(rusqlite::StatementStatus::VmStep)).ok()?,
        )?;
    }
    counted
}

pub(super) fn recognized(
    c: &Connection,
    sql: &str,
    parameters: &[Value],
    materialization: bool,
) -> Option<bool> {
    recognized_for_engine(c, sql, parameters, materialization, rusqlite::version())
}

fn recognized_for_engine(
    c: &Connection,
    sql: &str,
    parameters: &[Value],
    materialization: bool,
    version: &str,
) -> Option<bool> {
    if !matches!(version, "3.53.2" | "3.53.4") {
        return None;
    }
    let mut statement = c.prepare(&format!("EXPLAIN QUERY PLAN {sql}")).ok()?;
    let mut cursor = statement.query(params_from_iter(parameters)).ok()?;
    let mut index = false;
    let mut lookup = false;
    while let Some(row) = cursor.next().ok()? {
        let detail: String = row.get(3).ok()?;
        if matches!(
            detail.as_str(),
            "SEARCH k USING COVERING INDEX query_keys_value (kind=? AND field=? AND encoded=?)"
                | "SEARCH k USING COVERING INDEX query_keys_value (kind=? AND field=? AND encoded>?)"
                | "SEARCH k USING COVERING INDEX query_keys_value (kind=? AND field=? AND encoded<?)"
                | "SEARCH k USING COVERING INDEX query_keys_value (kind=? AND field=? AND encoded>? AND encoded<?)"
        ) {
            if index {
                return None;
            }
            index = true;
        } else if materialization
            && detail == "SEARCH r USING INDEX sqlite_autoindex_resources_1 (kind=? AND id=?)"
        {
            if lookup {
                return None;
            }
            lookup = true;
        } else {
            return None;
        }
    }
    Some(index && (lookup == materialization))
}

#[cfg(test)]
#[path = "probe/engine_profile_tests.rs"]
mod engine_profile_tests;
