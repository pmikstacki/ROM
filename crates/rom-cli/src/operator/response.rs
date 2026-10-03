//! Validate public response types and their correspondence before any output.
use crate::input::Request;
use rom::operator::{
    OperatorCapabilities, WorkControlRequest, WorkControlResult, WorkHandle, WorkPage, WorkQuery,
    WorkResponseLimits, WorkView,
};
use serde_json::Value;
use std::collections::BTreeSet;

fn bounds(records: usize) -> WorkResponseLimits {
    WorkResponseLimits {
        max_records: records,
        max_bytes: crate::sse::FRAME_LIMIT,
    }
}
pub fn valid(request: &Request, value: &Value) -> bool {
    if !value.is_object() {
        return false;
    }
    match request.route {
        "/work/capabilities" => serde_json::from_value::<OperatorCapabilities>(value.clone())
            .is_ok_and(|v| v.validate(&bounds(1)).is_ok()),
        "/work/list" => list(request, value),
        "/work/read" => read(request, value),
        "/work/control" => control(request, value),
        _ => false,
    }
}
fn read(request: &Request, value: &Value) -> bool {
    let Ok(handle) = serde_json::from_value::<WorkHandle>(request.body["handle"].clone()) else {
        return false;
    };
    serde_json::from_value::<WorkView>(value.clone())
        .is_ok_and(|v| v.validate(&bounds(1)).is_ok() && v.handle == handle)
}
fn list(request: &Request, value: &Value) -> bool {
    let Ok(query) = serde_json::from_value::<WorkQuery>(request.body.clone()) else {
        return false;
    };
    let Ok(page) = serde_json::from_value::<WorkPage>(value.clone()) else {
        return false;
    };
    if page.validate(&bounds(query.limit)).is_err() {
        return false;
    }
    let mut handles = BTreeSet::new();
    page.records.iter().all(|v| {
        handles.insert(&v.handle)
            && query.state.as_ref().is_none_or(|filter| &v.state == filter)
            && query
                .category
                .as_ref()
                .is_none_or(|filter| &v.category == filter)
            && query
                .definition
                .as_ref()
                .is_none_or(|filter| &v.definition.name == filter)
    })
}
fn control(request: &Request, value: &Value) -> bool {
    let Ok(request) = serde_json::from_value::<WorkControlRequest>(request.body.clone()) else {
        return false;
    };
    let Ok(result) = serde_json::from_value::<WorkControlResult>(value.clone()) else {
        return false;
    };
    result.validate_for(&request, &bounds(1)).is_ok()
}
