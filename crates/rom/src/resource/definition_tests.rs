use super::*;
use crate::{FieldDescriptor, Shape, json};
use std::panic::{AssertUnwindSafe, catch_unwind};

#[derive(Clone)]
struct Record {
    writable: bool,
}

impl Resource for Record {
    const KIND: &'static str = "read-policy-records";

    fn descriptor() -> Descriptor {
        Descriptor {
            kind: Self::KIND.into(),
            version: 1,
            fields: vec![FieldDescriptor {
                name: "writable".into(),
                shape: Shape::Bool,
            }],
        }
    }

    fn normalize_field(name: &str, value: Value) -> Result<Value> {
        if name == "writable" && value.is_boolean() {
            Ok(value)
        } else {
            Err(Error::invalid(Self::KIND, name))
        }
    }

    fn encode(&self) -> Value {
        json!({"writable": self.writable})
    }

    fn decode(value: Value) -> Result<Self> {
        assert_ne!(value, json!("decoder-panic"), "observable Resource decoder");
        Ok(Self {
            writable: value
                .get("writable")
                .and_then(Value::as_bool)
                .ok_or_else(|| Error::invalid(Self::KIND, "writable"))?,
        })
    }
}

fn actor(subject: &str) -> Actor {
    Actor::trusted("definition-tests", subject)
}

#[test]
fn explicit_read_rule_replaces_reads_and_keeps_row_dependent_writes() {
    let definition = Record::definition()
        .policy(|_, access, record| match access {
            Access::Read => panic!("explicit read must replace the opaque read callback"),
            Access::Write => record.writable,
        })
        .read_policy(|actor| actor.subject == "reader");
    let reader = actor("reader");
    let other = actor("other");
    assert_eq!(definition.uniform_read(&reader), Some(true));
    assert_eq!(definition.uniform_read(&other), Some(false));
    assert!(definition.allows(&reader, Access::Read, &json!({"writable": false})));
    assert!(!definition.allows(&other, Access::Read, &json!({"writable": true})));
    assert!(definition.allows(&other, Access::Write, &json!({"writable": true})));
    assert!(!definition.allows(&reader, Access::Write, &json!({"writable": false})));
    assert!(!Record::definition().read_policy(|_| true).allows(
        &reader,
        Access::Write,
        &json!({"writable": true})
    ));
}

#[test]
fn later_policy_restores_opaque_reads_and_later_read_policy_replaces_only_reads() {
    let reader = actor("reader");
    let value = json!({"writable": true});
    let opaque = Record::definition()
        .read_policy(|_| true)
        .policy(|_, access, _| matches!(access, Access::Write));
    assert_eq!(opaque.uniform_read(&reader), None);
    assert!(!opaque.allows(&reader, Access::Read, &value));
    assert!(opaque.allows(&reader, Access::Write, &value));

    let replaced = opaque.read_policy(|_| false).read_policy(|_| true);
    assert_eq!(replaced.uniform_read(&reader), Some(true));
    assert!(replaced.allows(&reader, Access::Read, &value));
    assert!(replaced.allows(&reader, Access::Write, &value));
    assert_eq!(Record::definition().uniform_read(&reader), None);
}

#[test]
fn only_explicit_whole_field_grant_sets_proof_and_field_override_clears_it() {
    let definition = Record::definition();
    assert!(!definition.uniform_fields());
    let definition = definition.field_policy(|_, _, _, _| true);
    assert!(!definition.uniform_fields());
    let definition = definition.allow_all_fields();
    assert!(definition.uniform_fields());
    let definition = definition
        .query_policy(|_, _| false)
        .sort_policy(|_, _| false);
    assert!(definition.uniform_fields());
    assert!(!definition.allows_query(&actor("reader"), "writable"));
    assert!(!definition.allows_sort(&actor("reader"), "writable"));
    let definition = definition.field_policy(|_, _, _, _| true);
    assert!(!definition.uniform_fields());
    assert!(definition.allow_all_fields().uniform_fields());
}

#[test]
fn explicit_reads_skip_decoder_but_writes_and_field_disclosure_keep_it() {
    let reader = actor("reader");
    let poison = json!("decoder-panic");
    let definition = Record::definition()
        .policy(|_, _, _| true)
        .read_policy(|_| true)
        .allow_all_fields();
    assert!(definition.allows(&reader, Access::Read, &poison));
    assert!(
        catch_unwind(AssertUnwindSafe(|| {
            definition.allows(&reader, Access::Write, &poison)
        }))
        .is_err()
    );
    assert!(
        catch_unwind(AssertUnwindSafe(|| {
            definition.allows_field(&reader, Access::Read, "writable", &poison)
        }))
        .is_err()
    );
    assert!(!definition.allows_field(&reader, Access::Read, "writable", &Value::Null));
    assert!(!definition.allows(&reader, Access::Write, &Value::Null));
    let opaque = definition.policy(|_, _, _| true);
    assert!(
        catch_unwind(AssertUnwindSafe(|| {
            opaque.allows(&reader, Access::Read, &poison)
        }))
        .is_err()
    );
}

#[test]
fn explicit_denial_never_calls_decoder_or_grants_field_access() {
    let definition = Record::definition().read_policy(|_| false);
    let reader = actor("reader");
    assert_eq!(definition.uniform_read(&reader), Some(false));
    assert!(!definition.allows(&reader, Access::Read, &json!("decoder-panic")));
    assert!(!definition.uniform_fields());
    assert!(!definition.allows_field(
        &reader,
        Access::Read,
        "writable",
        &json!({"writable": true})
    ));
}
