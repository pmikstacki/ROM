//! Disposable experiment: fixed scalar schema, no maintained ROM API or format.
use redb::{ReadableDatabase, ReadableTable};
use rusqlite::{params, params_from_iter, types::Value as SqlValue};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Note {
    Missing,
    Null,
    Value(String),
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Row {
    pub id: String,
    pub amount: u64,
    pub title: String,
    pub note: Note,
    pub visible: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    Amount,
    Title,
    Note,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Predicate {
    AmountGe(u64),
    TitleEq(String),
    NoteEq(Note),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Order(pub Field, pub bool); // true means descending; ID remains ascending.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Plan {
    pub filters: Vec<Predicate>,
    pub order: Vec<Order>,
}
pub fn normalize(mut plan: Plan) -> Result<Plan> {
    if plan.filters.len() > 32 || plan.order.len() > 4 {
        return Err("plan budget".into());
    }
    for p in &mut plan.filters {
        if let Predicate::TitleEq(v) = p {
            *v = v.trim().into();
        }
    }
    Ok(plan)
}
pub fn authorize(plan: &Plan, can_order_amount: bool) -> Result<()> {
    if !can_order_amount && plan.order.iter().any(|o| o.0 == Field::Amount) {
        return Err("sort denied".into());
    }
    Ok(())
}
fn matches(row: &Row, plan: &Plan) -> bool {
    plan.filters.iter().all(|p| match p {
        Predicate::AmountGe(n) => row.amount >= *n,
        Predicate::TitleEq(s) => row.title == *s,
        Predicate::NoteEq(n) => row.note == *n,
    })
}
pub fn compare(a: &Row, b: &Row, plan: &Plan) -> Ordering {
    for Order(field, desc) in &plan.order {
        let c = match field {
            Field::Amount => a.amount.cmp(&b.amount),
            Field::Title => a.title.as_bytes().cmp(b.title.as_bytes()),
            Field::Note => a.note.cmp(&b.note),
        };
        let c = if *desc { c.reverse() } else { c };
        if c != Ordering::Equal {
            return c;
        }
    }
    a.id.as_bytes().cmp(b.id.as_bytes())
}
#[derive(Default, Debug)]
pub struct Read {
    pub rows: Vec<Row>,
    pub candidates: usize,
    pub decoded_bytes: usize,
    pub vm_steps: i32,
}
impl Read {
    pub fn ids(&self) -> Vec<&str> {
        self.rows.iter().map(|r| r.id.as_str()).collect()
    }
}
pub fn oracle(rows: &[Row], plan: &Plan, after: Option<&Row>, limit: usize) -> Read {
    let mut out: Vec<_> = rows
        .iter()
        .filter(|r| {
            r.visible
                && matches(r, plan)
                && after.is_none_or(|a| compare(r, a, plan) == Ordering::Greater)
        })
        .cloned()
        .collect();
    out.sort_by(|a, b| compare(a, b, plan));
    out.truncate(limit);
    Read {
        rows: out,
        candidates: rows.len(),
        ..Read::default()
    }
}
#[cfg(feature = "generated")]
pub mod generated {
    include!(concat!(env!("OUT_DIR"), "/generated.rs"));
}
#[cfg(feature = "hybrid")]
pub mod hybrid {
    use super::*;
    use std::marker::PhantomData;
    pub struct TypedField<T>(Field, PhantomData<T>);
    pub const AMOUNT: TypedField<u64> = TypedField(Field::Amount, PhantomData);
    pub const TITLE: TypedField<String> = TypedField(Field::Title, PhantomData);
    impl TypedField<u64> {
        pub fn ge(&self, n: u64) -> Predicate {
            Predicate::AmountGe(n)
        }
    }
    impl TypedField<String> {
        pub fn eq(&self, s: &str) -> Predicate {
            Predicate::TitleEq(s.into())
        }
    }
    impl<T> TypedField<T> {
        pub fn asc(&self) -> Order {
            Order(self.0, false)
        }
        pub fn desc(&self) -> Order {
            Order(self.0, true)
        }
    }
    #[derive(Default)]
    pub struct Builder(Plan);
    impl Builder {
        pub fn new() -> Self {
            Self::default()
        }
        pub fn filter(mut self, p: Predicate) -> Self {
            self.0.filters.push(p);
            self
        }
        pub fn order(mut self, o: Order) -> Self {
            self.0.order.push(o);
            self
        }
        pub fn finish(self) -> Plan {
            normalize(self.0).unwrap()
        }
    }
}
#[cfg(feature = "runtime")]
pub fn compile_wire(kind: &str, value: &serde_json::Value) -> Result<Plan> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Wire {
        filters: Vec<Filter>,
        order: Vec<Sort>,
    }
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Filter {
        field: String,
        op: String,
        value: serde_json::Value,
    }
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Sort {
        field: String,
        descending: bool,
    }
    let number = match kind {
        "inventory" => "amount",
        "tickets" => "priority",
        _ => return Err("kind".into()),
    };
    let field = |name: &str| -> Result<Field> {
        match name {
            s if s == number => Ok(Field::Amount),
            "title" => Ok(Field::Title),
            "note" => Ok(Field::Note),
            _ => Err("field".into()),
        }
    };
    let wire: Wire = serde_json::from_value(value.clone())?;
    let mut plan = Plan::default();
    for f in wire.filters {
        plan.filters.push(match (field(&f.field)?, f.op.as_str()) {
            (Field::Amount, "ge") => Predicate::AmountGe(f.value.as_u64().ok_or("u64 required")?),
            (Field::Title, "eq") => {
                Predicate::TitleEq(f.value.as_str().ok_or("text required")?.into())
            }
            _ => return Err("operator".into()),
        });
    }
    for o in wire.order {
        plan.order.push(Order(field(&o.field)?, o.descending));
    }
    normalize(plan)
}
fn note_parts(note: &Note) -> (i64, &str) {
    match note {
        Note::Missing => (0, ""),
        Note::Null => (1, ""),
        Note::Value(s) => (2, s),
    }
}
fn components(plan: &Plan, anchor: Option<&Row>) -> Vec<(&'static str, bool, SqlValue)> {
    let mut out = Vec::new();
    for Order(field, desc) in &plan.order {
        match field {
            Field::Amount => out.push((
                "amount",
                *desc,
                anchor.map_or(SqlValue::Null, |a| {
                    SqlValue::Blob(a.amount.to_be_bytes().to_vec())
                }),
            )),
            Field::Title => out.push((
                "title COLLATE BINARY",
                *desc,
                anchor.map_or(SqlValue::Null, |a| SqlValue::Text(a.title.clone())),
            )),
            Field::Note => {
                let (rank, text) = anchor.map_or((0, ""), |a| note_parts(&a.note));
                out.push(("note_state", *desc, SqlValue::Integer(rank)));
                out.push((
                    "note_value COLLATE BINARY",
                    *desc,
                    SqlValue::Text(text.into()),
                ));
            }
        }
    }
    out.push((
        "id COLLATE BINARY",
        false,
        anchor.map_or(SqlValue::Null, |a| SqlValue::Text(a.id.clone())),
    ));
    out
}
fn sql(plan: &Plan, anchor: Option<&Row>) -> (String, Vec<SqlValue>) {
    let mut conditions = Vec::new();
    let mut values = Vec::new();
    for p in &plan.filters {
        match p {
            Predicate::AmountGe(n) => {
                conditions.push("amount >= ?".to_owned());
                values.push(SqlValue::Blob(n.to_be_bytes().to_vec()));
            }
            Predicate::TitleEq(s) => {
                conditions.push("title COLLATE BINARY = ?".into());
                values.push(SqlValue::Text(s.clone()));
            }
            Predicate::NoteEq(n) => {
                let (rank, text) = note_parts(n);
                conditions.push("note_state = ? AND note_value COLLATE BINARY = ?".into());
                values.extend([SqlValue::Integer(rank), SqlValue::Text(text.into())]);
            }
        }
    }
    let parts = components(plan, anchor);
    if anchor.is_some() {
        let mut alternatives = Vec::new();
        for i in 0..parts.len() {
            let mut terms = Vec::new();
            for (j, (column, desc, value)) in parts.iter().enumerate().take(i + 1) {
                terms.push(format!(
                    "{column} {} ?",
                    if j < i {
                        "="
                    } else if *desc {
                        "<"
                    } else {
                        ">"
                    }
                ));
                values.push(value.clone());
            }
            alternatives.push(format!("({})", terms.join(" AND ")));
        }
        conditions.push(format!("({})", alternatives.join(" OR ")));
    }
    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", conditions.join(" AND "))
    };
    let order = parts
        .iter()
        .map(|(c, d, _)| format!("{c} {}", if *d { "DESC" } else { "ASC" }))
        .collect::<Vec<_>>()
        .join(", ");
    (
        format!(
            "SELECT id,amount,title,note_state,note_value,visible FROM items{where_clause} ORDER BY {order}"
        ),
        values,
    )
}
pub struct Sqlite(rusqlite::Connection);
impl Sqlite {
    pub fn new(rows: &[Row], index: bool) -> Result<Self> {
        Self::initialize(rusqlite::Connection::open_in_memory()?, rows, index)
    }
    pub fn create_at(path: &std::path::Path, rows: &[Row], index: bool) -> Result<Self> {
        Self::initialize(rusqlite::Connection::open(path)?, rows, index)
    }
    pub fn reopen(path: &std::path::Path) -> Result<Self> {
        Ok(Self(rusqlite::Connection::open_with_flags(
            path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE,
        )?))
    }
    fn initialize(mut db: rusqlite::Connection, rows: &[Row], index: bool) -> Result<Self> {
        db.execute_batch("CREATE TABLE items(id TEXT PRIMARY KEY COLLATE BINARY,amount BLOB NOT NULL,title TEXT NOT NULL COLLATE BINARY,note_state INTEGER NOT NULL,note_value TEXT NOT NULL COLLATE BINARY,visible INTEGER NOT NULL);")?;
        db.execute_batch(
            "CREATE TABLE journal(seq INTEGER PRIMARY KEY,id TEXT NOT NULL,amount BLOB NOT NULL)",
        )?;
        {
            let tx = db.transaction()?;
            {
                let mut insert = tx.prepare("INSERT INTO items VALUES(?,?,?,?,?,?)")?;
                for r in rows {
                    let (state, note) = note_parts(&r.note);
                    insert.execute(params![
                        r.id,
                        r.amount.to_be_bytes().as_slice(),
                        r.title,
                        state,
                        note,
                        r.visible
                    ])?;
                }
            }
            tx.commit()?;
        }
        if index {
            db.execute_batch("CREATE INDEX amount_order ON items(amount DESC,title COLLATE BINARY ASC,id COLLATE BINARY ASC)")?;
        }
        Ok(Self(db))
    }
    pub fn mutate_amount(&mut self, id: &str, amount: u64) -> Result<()> {
        let tx = self.0.transaction()?;
        let bytes = amount.to_be_bytes();
        if tx.execute(
            "UPDATE items SET amount=? WHERE id=?",
            params![bytes.as_slice(), id],
        )? != 1
        {
            return Err("unknown row".into());
        }
        tx.execute(
            "INSERT INTO journal(id,amount) VALUES(?,?)",
            params![id, bytes.as_slice()],
        )?;
        tx.commit()?;
        Ok(())
    }
    pub fn journal(&self) -> Result<Vec<(i64, String, u64)>> {
        let mut statement = self
            .0
            .prepare("SELECT seq,id,amount FROM journal ORDER BY seq")?;
        let rows = statement.query_map([], |r| {
            let bytes: Vec<u8> = r.get(2)?;
            Ok((
                r.get(0)?,
                r.get(1)?,
                u64::from_be_bytes(bytes.try_into().expect("fixedwidth journal scalar")),
            ))
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }
    /// Raw bounded materialization: policy, predicates and requested ordering run once in the oracle.
    pub fn snapshot(&self, budget: usize) -> Result<Read> {
        let mut statement = self.0.prepare(
            "SELECT id,amount,title,note_state,note_value,visible FROM items ORDER BY id LIMIT ?",
        )?;
        let mut out = Read::default();
        {
            let mut cursor = statement.query([i64::try_from(budget.saturating_add(1))?])?;
            while let Some(row) = cursor.next()? {
                if out.candidates == budget {
                    return Err("candidate budget exhausted".into());
                }
                let row = decode_row(row)?;
                out.candidates += 1;
                out.decoded_bytes += serde_json::to_vec(&row)?.len();
                out.rows.push(row);
            }
        }
        out.vm_steps = statement.get_status(rusqlite::StatementStatus::VmStep);
        Ok(out)
    }
    pub fn query(
        &self,
        plan: &Plan,
        after: Option<&Row>,
        limit: usize,
        budget: usize,
    ) -> Result<Read> {
        if limit == 0 {
            return Ok(Read::default());
        }
        let (mut sql, mut values) = sql(plan, after);
        sql.push_str(" LIMIT ?");
        values.push(SqlValue::Integer(i64::try_from(budget.saturating_add(1))?));
        let mut statement = self.0.prepare(&sql)?;
        let mut result = Read::default();
        {
            let mut cursor = statement.query(params_from_iter(values))?;
            while let Some(row) = cursor.next()? {
                if result.candidates == budget {
                    return Err("candidate budget exhausted".into());
                }
                result.candidates += 1;
                let row = decode_row(row)?;
                result.decoded_bytes += serde_json::to_vec(&row)?.len();
                if row.visible {
                    result.rows.push(row);
                    if result.rows.len() == limit {
                        break;
                    }
                }
            }
        }
        result.vm_steps = statement.get_status(rusqlite::StatementStatus::VmStep);
        Ok(result)
    }
    pub fn wrong_limit_before_policy(&self, plan: &Plan, limit: usize) -> Result<Vec<Row>> {
        let (mut sql, mut values) = sql(plan, None);
        sql.push_str(" LIMIT ?");
        values.push(SqlValue::Integer(limit as i64));
        let mut statement = self.0.prepare(&sql)?;
        let mut cursor = statement.query(params_from_iter(values))?;
        let mut result = Vec::new();
        while let Some(r) = cursor.next()? {
            let r = decode_row(r)?;
            if r.visible {
                result.push(r);
            }
        }
        Ok(result)
    }
    pub fn explain(&self, plan: &Plan) -> Result<Vec<String>> {
        let (sql, values) = sql(plan, None);
        let mut s = self.0.prepare(&format!("EXPLAIN QUERY PLAN {sql}"))?;
        let rows = s.query_map(params_from_iter(values), |r| r.get(3))?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }
    pub fn version(&self) -> String {
        rusqlite::version().into()
    }
}
fn decode_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Row> {
    let bytes: Vec<u8> = row.get(1)?;
    let amount = u64::from_be_bytes(
        bytes
            .try_into()
            .expect("prototype generated fixedwidth scalar"),
    );
    let state: i64 = row.get(3)?;
    Ok(Row {
        id: row.get(0)?,
        amount,
        title: row.get(2)?,
        note: match state {
            0 => Note::Missing,
            1 => Note::Null,
            _ => Note::Value(row.get(4)?),
        },
        visible: row.get(5)?,
    })
}
const TABLE: redb::TableDefinition<&str, &[u8]> = redb::TableDefinition::new("prototype_rows");
pub struct Redb {
    db: redb::Database,
}
impl Redb {
    pub fn reopen(path: &std::path::Path) -> Result<Self> {
        Ok(Self {
            db: redb::Database::open(path)?,
        })
    }
    pub fn mutate_amount(&self, id: &str, amount: u64) -> Result<()> {
        let tx = self.db.begin_write()?;
        {
            let mut table = tx.open_table(TABLE)?;
            let mut row: Row =
                serde_json::from_slice(table.get(id)?.ok_or("unknown row")?.value())?;
            row.amount = amount;
            table.insert(id, serde_json::to_vec(&row)?.as_slice())?;
        }
        tx.commit()?;
        Ok(())
    }
    pub fn new(path: &std::path::Path, rows: &[Row]) -> Result<Self> {
        let db = redb::Database::create(path)?;
        let tx = db.begin_write()?;
        {
            let mut t = tx.open_table(TABLE)?;
            for row in rows {
                t.insert(row.id.as_str(), serde_json::to_vec(row)?.as_slice())?;
            }
        }
        tx.commit()?;
        Ok(Self { db })
    }
    pub fn query(
        &self,
        plan: &Plan,
        anchor: Option<&Row>,
        limit: usize,
        budget: usize,
    ) -> Result<Read> {
        let tx = self.db.begin_read()?;
        let table = tx.open_table(TABLE)?;
        let mut rows = Vec::new();
        let mut bytes = 0;
        for item in table.iter()? {
            if rows.len() == budget {
                return Err("candidate budget exhausted".into());
            }
            let (_, v) = item?;
            bytes += v.value().len();
            rows.push(serde_json::from_slice(v.value())?);
        }
        let mut out = oracle(&rows, plan, anchor, limit);
        out.decoded_bytes = bytes;
        Ok(out)
    }
}
pub fn dataset(n: usize) -> Vec<Row> {
    (0..n)
        .map(|i| Row {
            id: format!("{i:08}"),
            amount: i as u64,
            title: if i % 13 == 0 {
                "é".into()
            } else {
                format!("title-{:03}", i % 101)
            },
            note: match i % 3 {
                0 => Note::Missing,
                1 => Note::Null,
                _ => Note::Value(format!("note-{}", i % 7)),
            },
            visible: i % 10 == 0,
        })
        .collect()
}
