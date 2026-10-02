use resource_contract::{validate_field, Descriptor, FieldDescriptor, Resource, ValidationError};
pub struct Manual {
    pub enabled: bool,
    pub note: Option<String>,
}
impl Resource for Manual {
    const NAME: &'static str = "manual";
    fn descriptor() -> Descriptor {
        Descriptor {
            name: Self::NAME,
            fields: vec![
                FieldDescriptor::of::<bool>("enabled"),
                FieldDescriptor::of::<Option<String>>("note"),
            ],
        }
    }
    fn validate(&self) -> Result<(), ValidationError> {
        validate_field(Self::NAME, "enabled", &self.enabled)?;
        validate_field(Self::NAME, "note", &self.note)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use resource_contract::Shape;
    #[test]
    fn same_contract_without_derive_feature() {
        fn inspect<T: Resource>(value: &T) -> Descriptor {
            value.validate().unwrap();
            T::descriptor()
        }
        let descriptor = inspect(&Manual {
            enabled: false,
            note: None,
        });
        assert_eq!(
            descriptor.fields[1].shape,
            Shape::Nullable(Box::new(Shape::Text))
        );
    }
}
