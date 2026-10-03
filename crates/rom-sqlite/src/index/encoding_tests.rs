use super::encoding::encode;
use rom::{Shape, json};
fn ordered(shape: Shape, values: &[rom::Value]) {
    let keys: Vec<_> = values
        .iter()
        .map(|v| encode(&shape, Some(v)).unwrap())
        .collect();
    assert!(keys.windows(2).all(|p| p[0] < p[1]), "{shape:?}");
}
#[test]
fn exact_numeric_and_text_boundaries() {
    ordered(
        Shape::U64,
        &[
            json!(0),
            json!(1),
            json!(i64::MAX as u64),
            json!((i64::MAX as u64) + 1),
            json!(u64::MAX),
        ],
    );
    ordered(
        Shape::I64,
        &[
            json!(i64::MIN),
            json!(-1),
            json!(0),
            json!(1),
            json!(i64::MAX),
        ],
    );
    ordered(
        Shape::F64,
        &[
            json!(-f64::MAX),
            json!(-1.0),
            json!(-f64::from_bits(1)),
            json!(0.0),
            json!(f64::from_bits(1)),
            json!(1.0),
            json!(f64::from_bits(1.0f64.to_bits() + 1)),
            json!(f64::MAX),
        ],
    );
    ordered(
        Shape::String,
        &[
            json!(""),
            json!("\0"),
            json!("a"),
            json!("a\0"),
            json!("aa"),
            json!("é"),
            json!("雪"),
        ],
    );
    ordered(Shape::Bool, &[json!(false), json!(true)]);
}
#[test]
fn float_equality_matches_core_and_presence_is_distinct() {
    assert_eq!(
        encode(&Shape::F64, Some(&json!(-0.0))),
        encode(&Shape::F64, Some(&json!(0.0)))
    );
    assert_eq!(
        encode(&Shape::F64, Some(&json!(1))),
        encode(&Shape::F64, Some(&json!(1.0)))
    );
    let shape = Shape::Optional(Box::new(Shape::Nullable(Box::new(Shape::U64))));
    let absent = encode(&shape, None).unwrap();
    let null = encode(&shape, Some(&json!(null))).unwrap();
    let value = encode(&shape, Some(&json!(0))).unwrap();
    assert!(absent < null && null < value);
    assert_eq!(absent, vec![0]);
    assert_eq!(null, vec![1]);
}
#[test]
fn invalid_values_and_non_scalar_shapes_fail_closed() {
    for (shape, value) in [
        (Shape::U64, json!(-1)),
        (Shape::I64, json!(u64::MAX)),
        (Shape::Bool, json!(0)),
        (Shape::String, json!(null)),
        (Shape::Optional(Box::new(Shape::U64)), json!(null)),
        (Shape::Enum(vec!["ok".into()]), json!("bad")),
        (Shape::Reference { kind: "x".into() }, json!("")),
    ] {
        assert!(encode(&shape, Some(&value)).is_err(), "{shape:?}");
    }
    assert!(encode(&Shape::U64, None).is_err());
    for shape in [
        Shape::List(Box::new(Shape::U64)),
        Shape::Map(Box::new(Shape::U64)),
    ] {
        assert!(!shape.is_scalar());
        assert!(encode(&shape, Some(&json!([]))).is_err());
    }
}
