pub fn decode_input_member<T: crate::Field>(value: Option<crate::Value>) -> crate::Result<T> {
    crate::resource::validate_shape(&T::shape(), 0, None)?;
    match value {
        Some(value) => T::decode(value),
        None => T::decode_missing(),
    }
}
