//! Throwaway shared contract. No procedural macro dependency belongs here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Shape {
    Boolean,
    Integer,
    Text,
    Custom(&'static str),
    Nullable(Box<Shape>),
    List(Box<Shape>),
}

pub trait Field {
    fn shape() -> Shape;
    fn validate(&self) -> Result<(), String> {
        Ok(())
    }
}
impl Field for bool {
    fn shape() -> Shape {
        Shape::Boolean
    }
}
impl Field for i64 {
    fn shape() -> Shape {
        Shape::Integer
    }
}
impl Field for String {
    fn shape() -> Shape {
        Shape::Text
    }
}
impl<T: Field> Field for Option<T> {
    fn shape() -> Shape {
        Shape::Nullable(Box::new(T::shape()))
    }
    fn validate(&self) -> Result<(), String> {
        self.as_ref().map_or(Ok(()), T::validate)
    }
}
impl<T: Field> Field for Vec<T> {
    fn shape() -> Shape {
        Shape::List(Box::new(T::shape()))
    }
    fn validate(&self) -> Result<(), String> {
        for (index, value) in self.iter().enumerate() {
            value
                .validate()
                .map_err(|message| format!("item[{index}]: {message}"))?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldDescriptor {
    pub name: &'static str,
    pub shape: Shape,
}
impl FieldDescriptor {
    pub fn of<T: Field>(name: &'static str) -> Self {
        Self {
            name,
            shape: T::shape(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Descriptor {
    pub name: &'static str,
    pub fields: Vec<FieldDescriptor>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidationError {
    pub resource: &'static str,
    pub field: &'static str,
    pub message: String,
}
impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "resource `{}`, field `{}`: {}",
            self.resource, self.field, self.message
        )
    }
}
impl std::error::Error for ValidationError {}

pub trait Resource {
    const NAME: &'static str;
    fn descriptor() -> Descriptor;
    fn validate(&self) -> Result<(), ValidationError>;
}
pub fn validate_field<T: Field>(
    resource: &'static str,
    field: &'static str,
    value: &T,
) -> Result<(), ValidationError> {
    value.validate().map_err(|message| ValidationError {
        resource,
        field,
        message,
    })
}
