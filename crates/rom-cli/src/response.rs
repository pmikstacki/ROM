//! Validate response shape, requested Resource scope, and journal continuation.
use crate::input::Request;
use serde_json::Value;

fn view(v: &Value) -> bool {
    v.is_object()
        && v.get("key").is_some_and(|k| {
            k.get("kind").is_some_and(Value::is_string) && k.get("id").is_some_and(Value::is_string)
        })
        && v.get("revision").and_then(Value::as_u64).is_some()
        && v.get("value").is_some_and(|v| v.is_null() || v.is_object())
}
fn cursor(v: &Value) -> bool {
    serde_json::from_value::<rom::JournalCursor>(v.clone()).is_ok()
}
fn valid_shape(route: &str, v: &Value) -> bool {
    match route {
        "/invoke" | "/read" => view(v),
        "/query" | "/live" => v.as_array().is_some_and(|rows| rows.iter().all(view)),
        "/journal/head" => cursor(v),
        "/journal" | "/subscribe" => {
            v.get("cursor").is_some_and(cursor)
                && v.get("events")
                    .and_then(Value::as_array)
                    .is_some_and(|events| {
                        events.iter().all(|e| {
                            e.get("position").and_then(Value::as_u64).is_some()
                                && e.get("view").is_some_and(view)
                        })
                    })
        }
        "/discover" => {
            v.get("version").and_then(Value::as_u64) == Some(1)
                && v.get("resources")
                    .and_then(Value::as_array)
                    .is_some_and(|rs| {
                        rs.iter().all(|r| {
                            r.get("kind").is_some_and(Value::is_string)
                                && r.get("version").and_then(Value::as_u64).is_some()
                                && r.get("fields").and_then(Value::as_array).is_some_and(|fs| {
                                    fs.iter().all(|f| {
                                        f.get("name").is_some_and(Value::is_string)
                                            && f.get("shape").is_some_and(Value::is_object)
                                    })
                                })
                                && r.get("actions")
                                    .and_then(Value::as_array)
                                    .is_some_and(|xs| xs.iter().all(Value::is_string))
                        })
                    })
        }
        _ => false,
    }
}

pub(crate) fn prior(request: &Request) -> Option<rom::JournalCursor> {
    request
        .body
        .get("after")
        .and_then(|value| serde_json::from_value(value.clone()).ok())
}
// Shape alone cannot correlate a response with this invocation or journal scope.
pub(crate) fn valid(request: &Request, value: &Value, after: Option<&rom::JournalCursor>) -> bool {
    if !valid_shape(request.route, value) {
        return false;
    }
    let kind = request.body.get("kind");
    match request.route {
        "/read" | "/invoke" => {
            value["key"].get("kind") == kind && value["key"].get("id") == request.body.get("id")
        }
        "/query" | "/live" => value
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["key"].get("kind") == kind),
        "/journal/head" => value.get("kind") == kind,
        "/journal" | "/subscribe" => {
            let cursor: rom::JournalCursor =
                serde_json::from_value(value["cursor"].clone()).unwrap();
            if Some(Value::String(cursor.kind.clone())).as_ref() != kind
                || after.is_some_and(|old| {
                    old.kind != cursor.kind
                        || old.generation != cursor.generation
                        || old.position > cursor.position
                })
            {
                return false;
            }
            let mut position = after.map_or(0, |old| old.position);
            for event in value["events"].as_array().unwrap() {
                let next = event["position"].as_u64().unwrap();
                if next <= position
                    || next > cursor.position
                    || event["view"]["key"].get("kind") != kind
                {
                    return false;
                }
                position = next;
            }
            true
        }
        _ => true,
    }
}
