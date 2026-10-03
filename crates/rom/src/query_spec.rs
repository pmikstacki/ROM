use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Equality {
    pub field: String,
    pub value: Value,
    #[serde(default)]
    pub absent: bool,
}
/// Conjunctive predicates and moving-view ordering. No snapshot is retained.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuerySpec {
    #[serde(default)]
    pub comparisons: Vec<Comparison>,
    #[serde(default)]
    pub order: Vec<OrderBy>,
    #[serde(default)]
    pub after: Option<QueryAnchor>,
    #[serde(default)]
    pub filters: Vec<Equality>,
    #[serde(default)]
    pub after_id: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}
impl QuerySpec {
    pub fn all() -> Self {
        Self::default()
    }
    pub fn equal(field: impl Into<String>, value: Value) -> Self {
        Self::all().and(field, value)
    }
    pub fn and(mut self, field: impl Into<String>, value: Value) -> Self {
        self.filters.push(Equality {
            field: field.into(),
            value,
            absent: false,
        });
        self
    }
    pub fn absent(field: impl Into<String>) -> Self {
        Self::all().and_absent(field)
    }
    pub fn and_absent(mut self, field: impl Into<String>) -> Self {
        self.filters.push(Equality {
            field: field.into(),
            value: Value::Null,
            absent: true,
        });
        self
    }
    pub fn after_id(mut self, id: impl Into<String>) -> Self {
        self.after_id = Some(id.into());
        self
    }
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompareOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Comparison {
    pub field: String,
    pub op: CompareOp,
    pub value: Value,
    #[serde(default)]
    pub absent: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Asc,
    Desc,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrderBy {
    pub field: String,
    pub direction: Direction,
}
/// Explicit missing differs from a present JSON null.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "state",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum AnchorValue {
    Missing,
    Value(Value),
}
/// A client-chosen moving boundary, never an authority or server-issued credential.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueryAnchor {
    pub version: u32,
    pub kind: String,
    pub schema_version: u32,
    pub filters: Vec<Equality>,
    pub comparisons: Vec<Comparison>,
    pub order: Vec<OrderBy>,
    pub id: String,
    pub values: Vec<AnchorValue>,
}
impl QuerySpec {
    pub fn compare(mut self, field: impl Into<String>, op: CompareOp, value: Value) -> Self {
        self.comparisons.push(Comparison {
            field: field.into(),
            op,
            value,
            absent: false,
        });
        self
    }
    pub fn order_by(mut self, field: impl Into<String>, direction: Direction) -> Self {
        self.order.push(OrderBy {
            field: field.into(),
            direction,
        });
        self
    }
    pub fn after(mut self, anchor: QueryAnchor) -> Self {
        self.after = Some(anchor);
        self
    }
}
