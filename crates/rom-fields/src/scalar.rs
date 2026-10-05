/// Generate the common string Field boundary; each parser owns its guarantees.
macro_rules! string_field {
    ($name:ident, $identity:literal, $parse:path, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub struct $name(String);
        impl $name {
            /// Validate and canonicalize a candidate string.
            pub fn new(value: impl AsRef<str>) -> rom::Result<Self> {
                $parse(value.as_ref()).map(Self)
            }
            /// Return the exact canonical wire string.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
        impl std::str::FromStr for $name {
            type Err = rom::Error;
            fn from_str(value: &str) -> rom::Result<Self> {
                Self::new(value)
            }
        }
        impl rom::Field for $name {
            fn shape() -> rom::Shape {
                rom::Shape::String
            }
            fn codec_identity() -> Option<rom::CodecIdentity> {
                Some(rom::CodecIdentity {
                    name: $identity.into(),
                    version: 1,
                })
            }
            fn encode(&self) -> rom::Value {
                rom::Value::String(self.0.clone())
            }
            fn decode(value: rom::Value) -> rom::Result<Self> {
                match value {
                    rom::Value::String(value) => Self::new(value),
                    _ => Err(rom::Error::invalid("input", $identity)),
                }
            }
        }
    };
}
pub(crate) use string_field;
pub(crate) fn invalid(identity: &str) -> rom::Error {
    rom::Error::invalid("input", identity)
}
