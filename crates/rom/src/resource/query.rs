//! Typed query authoring over the canonical query representation.
use crate::{
    CompareOp, Direction, Error, Field, QuerySpec, Resource, Result, Snapshot, query_eval,
};
use std::marker::PhantomData;

#[derive(Clone)]
pub struct FieldRef<R, T> {
    pub(crate) name: &'static str,
    marker: PhantomData<fn() -> (R, T)>,
}
impl<R: Resource, T: Field> FieldRef<R, T> {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            marker: PhantomData,
        }
    }
    pub fn equals(self, value: T) -> Query<R> {
        Query {
            spec: if value.is_present() {
                QuerySpec::equal(self.name, Field::encode(&value))
            } else {
                QuerySpec::absent(self.name)
            },
            marker: PhantomData,
        }
    }
    pub fn compare(self, op: CompareOp, value: T) -> Query<R> {
        let mut spec = QuerySpec::all().compare(self.name, op, Field::encode(&value));
        spec.comparisons[0].absent = !value.is_present();
        Query {
            spec,
            marker: PhantomData,
        }
    }
    pub fn not_equals(self, value: T) -> Query<R> {
        self.compare(CompareOp::Ne, value)
    }
    pub fn less_than(self, value: T) -> Query<R> {
        self.compare(CompareOp::Lt, value)
    }
    pub fn at_most(self, value: T) -> Query<R> {
        self.compare(CompareOp::Le, value)
    }
    pub fn greater_than(self, value: T) -> Query<R> {
        self.compare(CompareOp::Gt, value)
    }
    pub fn at_least(self, value: T) -> Query<R> {
        self.compare(CompareOp::Ge, value)
    }
}
#[derive(Clone)]
pub struct Query<R> {
    pub(crate) spec: QuerySpec,
    marker: PhantomData<fn() -> R>,
}
impl<R: Resource> Query<R> {
    /// Select all currently authorized rows, subject to the usual snapshot bounds.
    pub fn all() -> Self {
        Self {
            spec: QuerySpec::all(),
            marker: PhantomData,
        }
    }
    pub fn and<T: Field>(mut self, field: FieldRef<R, T>, value: T) -> Self {
        self.spec = if value.is_present() {
            self.spec.and(field.name, Field::encode(&value))
        } else {
            self.spec.and_absent(field.name)
        };
        self
    }
    pub fn after_id(mut self, id: impl Into<String>) -> Self {
        self.spec = self.spec.after_id(id);
        self
    }
    pub fn limit(mut self, limit: usize) -> Self {
        self.spec = self.spec.limit(limit);
        self
    }
    pub fn spec(&self) -> &QuerySpec {
        &self.spec
    }
    /// Conjoin predicates only; a nested page/order is rejected instead of discarded.
    pub fn and_where(mut self, other: Self) -> Result<Self> {
        if !other.spec.order.is_empty()
            || other.spec.limit.is_some()
            || other.spec.after.is_some()
            || other.spec.after_id.is_some()
        {
            return Err(Error::invalid(R::KIND, "predicate composition"));
        }
        self.spec.filters.extend(other.spec.filters);
        self.spec.comparisons.extend(other.spec.comparisons);
        Ok(self)
    }
    pub fn order_by<T: Field>(mut self, field: FieldRef<R, T>, direction: Direction) -> Self {
        self.spec = self.spec.order_by(field.name, direction);
        self
    }
    pub fn after_snapshot(mut self, snapshot: &Snapshot<R>) -> Result<Self> {
        if self.spec.after_id.is_some() {
            return Err(Error::invalid(R::KIND, "after_id"));
        }
        let value = snapshot.value.as_ref().ok_or(Error::Missing)?.encode();
        let descriptor = R::descriptor();
        let plan = query_eval::normalize(&descriptor, &self.spec, R::normalize_field)?;
        self.spec.after = Some(query_eval::make_anchor(
            &descriptor,
            &plan,
            &snapshot.id,
            &value,
            R::normalize_field,
        )?);
        Ok(self)
    }
}
impl<R: Resource> Default for Query<R> {
    fn default() -> Self {
        Self::all()
    }
}
