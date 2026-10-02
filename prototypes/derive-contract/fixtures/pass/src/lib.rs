#![allow(dead_code)]
use resource_contract::{Field, Resource, Shape};

struct UpperCode(String);
impl Field for UpperCode {
    fn shape() -> Shape {
        Shape::Custom("upper-code")
    }
    fn validate(&self) -> Result<(), String> {
        if !self.0.is_empty() && self.0.bytes().all(|c| c.is_ascii_uppercase()) {
            Ok(())
        } else {
            Err("use nonempty uppercase ASCII letters".into())
        }
    }
}
type MaybeCode = Option<UpperCode>;
type Nested = Vec<Option<Vec<MaybeCode>>>;

#[derive(Resource)]
#[resource(name = "sensor")]
struct Sensor {
    #[resource(rename = "deviceCode")]
    code: MaybeCode,
    groups: Nested,
    enabled: bool,
}
// The same Rust identifier in separate modules creates no generated global symbols.
mod first {
    #[derive(resource_contract::Resource)]
    #[resource(name = "first")]
    pub struct Same {
        pub value: bool,
    }
}
mod second {
    #[derive(resource_contract::Resource)]
    #[resource(name = "second")]
    pub struct Same {
        pub value: i64,
    }
}
#[derive(Resource)]
#[resource(name = "generic")]
struct Generic<T> {
    value: T,
}

#[cfg(test)]
mod tests {
    use super::*;
    use resource_contract::{Descriptor, FieldDescriptor};
    #[test]
    fn custom_alias_option_and_nested_structure_preserved_by_traits() {
        assert_eq!(
            Sensor::descriptor(),
            Descriptor {
                name: "sensor",
                fields: vec![
                    FieldDescriptor {
                        name: "deviceCode",
                        shape: Shape::Nullable(Box::new(Shape::Custom("upper-code")))
                    },
                    FieldDescriptor {
                        name: "groups",
                        shape: Shape::List(Box::new(Shape::Nullable(Box::new(Shape::List(
                            Box::new(Shape::Nullable(Box::new(Shape::Custom("upper-code"))))
                        )))))
                    },
                    FieldDescriptor {
                        name: "enabled",
                        shape: Shape::Boolean
                    },
                ]
            }
        );
        let good = Sensor {
            code: None,
            groups: vec![Some(vec![Some(UpperCode("ABC".into())), None])],
            enabled: false,
        };
        assert_eq!(good.validate(), Ok(()));
    }
    #[test]
    fn runtime_value_error_names_resource_and_externally_named_field() {
        let bad = Sensor {
            code: Some(UpperCode("lower".into())),
            groups: vec![],
            enabled: false,
        };
        assert_eq!(
            bad.validate().unwrap_err().to_string(),
            "resource `sensor`, field `deviceCode`: use nonempty uppercase ASCII letters"
        );
        let nested = Sensor {
            code: None,
            groups: vec![Some(vec![Some(UpperCode("bad".into()))])],
            enabled: false,
        };
        assert!(nested
            .validate()
            .unwrap_err()
            .to_string()
            .contains("field `groups`: item[0]: item[0]"));
    }
    #[test]
    fn module_names_do_not_collide_and_generic_field_bounds_work() {
        assert_ne!(
            first::Same::descriptor().name,
            second::Same::descriptor().name
        );
        assert_eq!(Generic::<i64>::descriptor().fields[0].shape, Shape::Integer);
    }
}
