use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Equality {
    pub field: String,
    pub value: Value,
    #[serde(default)]
    pub absent: bool,
}
/// Conjunctive equality and moving-view ID keyset pagination. No snapshot is retained.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuerySpec {
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
impl Runtime {
    pub(crate) fn select_rows(
        &self,
        actor: &Actor,
        kind: &str,
        query: &QuerySpec,
    ) -> Result<Vec<Row>> {
        if query.filters.len() > 32
            || query
                .limit
                .is_some_and(|n| n == 0 || n > self.0.limits.snapshot_rows)
        {
            return Err(Error::TooLarge);
        }
        check_query_size(&(kind, query), self.0.limits.command_bytes)?;
        let def = self.0.registry.get(kind).ok_or(Error::Unregistered)?;
        let descriptor = def.descriptor();
        let mut filters = Vec::with_capacity(query.filters.len());
        for filter in &query.filters {
            let field = descriptor
                .fields
                .iter()
                .find(|f| f.name == filter.field)
                .ok_or_else(|| Error::invalid(kind, &filter.field))?;
            if !def.allows_query(actor, &filter.field) {
                return Err(Error::Denied);
            }
            if filter.absent {
                if !filter.value.is_null() || !matches!(field.shape, Shape::Optional(_)) {
                    return Err(Error::invalid(kind, &filter.field));
                }
                filters.push(filter.clone());
                continue;
            }
            if !matches_shape(&filter.value, &field.shape) {
                return Err(Error::invalid(kind, &filter.field));
            }
            let value = def.normalize_field(&filter.field, filter.value.clone())?;
            if !matches_shape(&value, &field.shape) {
                return Err(Error::invalid(kind, &filter.field));
            }
            filters.push(Equality {
                field: filter.field.clone(),
                value,
                absent: false,
            });
        }
        let mut rows = self.0.storage.snapshot(
            kind,
            self.0.limits.snapshot_rows,
            self.0.limits.snapshot_bytes,
        )?;
        if rows.len() > self.0.limits.snapshot_rows {
            return Err(Error::TooLarge);
        }
        let mut bytes = 0usize;
        for row in &rows {
            if row.key.kind != kind {
                return Err(Error::Storage);
            }
            bytes = bytes
                .checked_add(serde_json::to_vec(row).map_err(|_| Error::Storage)?.len())
                .ok_or(Error::TooLarge)?;
            if bytes > self.0.limits.snapshot_bytes {
                return Err(Error::TooLarge);
            }
        }
        rows.sort_by(|a, b| a.key.id.cmp(&b.key.id));
        if rows.windows(2).any(|pair| pair[0].key.id == pair[1].key.id) {
            return Err(Error::Storage);
        }
        Ok(rows
            .into_iter()
            .filter(|row| {
                query.after_id.as_ref().is_none_or(|id| row.key.id > *id)
                    && row.value.as_ref().is_some_and(|v| {
                        def.allows(actor, Access::Read, v)
                            && filters.iter().all(|f| {
                                if f.absent {
                                    v.get(&f.field).is_none()
                                } else {
                                    v.get(&f.field) == Some(&f.value)
                                }
                            })
                    })
            })
            .take(query.limit.unwrap_or(self.0.limits.snapshot_rows))
            .collect())
    }
}
fn check_query_size(value: &impl Serialize, limit: usize) -> Result<()> {
    struct Counter(usize);
    impl std::io::Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0 = self
                .0
                .checked_sub(bytes.len())
                .ok_or_else(|| std::io::Error::other("query limit"))?;
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    serde_json::to_writer(Counter(limit), value).map_err(|_| Error::TooLarge)
}
