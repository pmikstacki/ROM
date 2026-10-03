use rom::{Error, Field, Result, Shape, Value};

#[derive(Clone, Debug, PartialEq)]
pub struct Code(String);
impl Field for Code {
    fn shape() -> Shape {
        Shape::String
    }
    fn encode(&self) -> Value {
        Value::String(self.0.clone())
    }
    fn decode(value: Value) -> Result<Self> {
        let input = value.as_str().ok_or(Error::Denied)?.trim();
        if input.is_empty()
            || input.len() > 12
            || !input
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        {
            return Err(Error::Denied);
        }
        Ok(Self(input.to_ascii_uppercase()))
    }
}
