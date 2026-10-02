//! THROWAWAY: a tiny declarative surface, not a proposed final macro API.
use serde::Serialize;
use serde_json::{json, Value};

pub trait FieldType {
    const NAME: &'static str;
    const NULLABLE: bool = false;
    fn accepts(value: &Value) -> bool;
}
impl FieldType for bool {
    const NAME: &'static str = "boolean";
    fn accepts(v: &Value) -> bool {
        v.is_boolean()
    }
}
impl FieldType for i64 {
    const NAME: &'static str = "integer";
    fn accepts(v: &Value) -> bool {
        v.is_i64()
    }
}
impl FieldType for f64 {
    const NAME: &'static str = "number";
    fn accepts(v: &Value) -> bool {
        v.is_number()
    }
}
impl FieldType for String {
    const NAME: &'static str = "text";
    fn accepts(v: &Value) -> bool {
        v.is_string()
    }
}
impl<T: FieldType> FieldType for Option<T> {
    const NAME: &'static str = T::NAME;
    const NULLABLE: bool = true;
    fn accepts(v: &Value) -> bool {
        v.is_null() || T::accepts(v)
    }
}
// A custom semantic field is an ordinary Rust extension, used by a declaration.
pub struct UpperCode;
impl FieldType for UpperCode {
    const NAME: &'static str = "upper-code";
    fn accepts(v: &Value) -> bool {
        v.as_str().is_some_and(|s| {
            !s.is_empty() && s.len() <= 16 && s.bytes().all(|c| c.is_ascii_uppercase())
        })
    }
}

#[derive(Clone, Serialize)]
pub struct Field {
    pub name: &'static str,
    pub field_type: &'static str,
    pub nullable: bool,
    // All fields must be present in a full document. Nullable != omitted.
    #[serde(skip)]
    pub accepts: fn(&Value) -> bool,
}
impl Field {
    fn of<T: FieldType>(name: &'static str) -> Self {
        Self {
            name,
            field_type: T::NAME,
            nullable: T::NULLABLE,
            accepts: T::accepts,
        }
    }
}
#[derive(Clone, Serialize)]
pub struct Reaction {
    pub name: &'static str,
    pub when_field: &'static str,
    pub equals: Value,
    pub target_kind: &'static str,
    pub id_prefix: &'static str,
    pub input: Value,
}
#[derive(Clone, Serialize)]
pub struct Descriptor {
    pub kind: &'static str,
    pub fields: Vec<Field>,
    pub actions: Vec<(&'static str, Value)>,
    pub reactions: Vec<Reaction>,
}
impl Descriptor {
    pub fn validate(&self, value: &Value, partial: bool) -> Result<(), String> {
        let object = value.as_object().ok_or("input must be an object")?;
        for (name, value) in object {
            let field = self
                .fields
                .iter()
                .find(|f| f.name == name)
                .ok_or_else(|| format!("unknown field {name}"))?;
            if !(field.accepts)(value) {
                return Err(format!(
                    "{name} must be {} (null only for nullable declarations)",
                    field.field_type
                ));
            }
        }
        if !partial {
            for field in &self.fields {
                if !object.contains_key(field.name) {
                    return Err(format!("missing field {}", field.name));
                }
            }
        }
        Ok(())
    }
}

macro_rules! resources {
    ($( $kind:ident { $( $field:ident : $ty:ty ),* $(,)? }
        actions { $( $action:ident => $patch:expr ),* $(,)? }
        reactions [ $( $reaction:expr ),* $(,)? ]; )*) => {
        pub fn registry() -> Vec<Descriptor> {
            vec![$(Descriptor {
                kind: stringify!($kind),
                fields: vec![$(Field::of::<$ty>(stringify!($field))),*],
                actions: vec![$((stringify!($action), $patch)),*],
                reactions: vec![$($reaction),*],
            }),*]
        }
    };
}

// Each block alone supplies CRUD, schema, persistence, action routing and stream eligibility.
resources! {
    thermostat { enabled: bool, target: i64, note: Option<String> }
    actions { disable => json!({"enabled": false}) }
    reactions [Reaction {
        name: "disabled-task", when_field: "enabled", equals: json!(false),
        target_kind: "task", id_prefix: "disabled",
        input: json!({"title": "Inspect disabled room", "done": false, "note": null}),
    }];

    task { title: String, done: bool, note: Option<String> }
    actions { complete => json!({"done": true}) }
    reactions [];

    // Third, unrelated kind: exactly these three declaration lines; no runtime edits.
    sensor { code: UpperCode, reading: f64 }
    actions {}
    reactions [];
}
