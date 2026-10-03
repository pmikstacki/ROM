//! Resource-value transformation shared by offline maintenance of all row copies.
use super::{Key, Result, Row, Value};

impl Row {
    /// Maintenance only: map the live value and retained deletion authorization value.
    /// Keys, revisions and source provenance are preserved. The original row is unchanged
    /// if a transform fails; callback side effects are outside this guarantee.
    pub fn map_resource_values(
        &self,
        transform: &mut impl FnMut(&Key, &Value) -> Result<Value>,
    ) -> Result<Row> {
        let mut mapped = self.clone();
        mapped.value = self
            .value
            .as_ref()
            .map(|value| transform(&self.key, value))
            .transpose()?;
        mapped.protected.deletion_authorization = self
            .protected
            .deletion_authorization
            .as_ref()
            .map(|value| transform(&self.key, value))
            .transpose()?;
        Ok(mapped)
    }
}
