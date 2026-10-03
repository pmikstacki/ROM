use crate::{
    ConformanceResult,
    error::{check, observe},
};
pub struct CodecCase<F> {
    pub input: rom::Value,
    pub canonical: rom::Value,
    pub expected: F,
}
pub fn codec<F: rom::Field + PartialEq>(
    cases: &[CodecCase<F>],
    invalid: &[rom::Value],
) -> ConformanceResult {
    for case in cases {
        let decoded = observe(F::decode(case.input.clone()), "field.decode")?;
        check(decoded == case.expected, "field.typed")?;
        check(decoded.encode() == case.canonical, "field.canonical")?;
        check(case.expected.encode() == case.canonical, "field.expected")?;
        let canonical = observe(F::decode(case.canonical.clone()), "field.roundtrip.decode")?;
        check(canonical == case.expected, "field.roundtrip.typed")?;
        check(
            canonical.encode() == case.canonical,
            "field.roundtrip.canonical",
        )?;
    }
    for value in invalid {
        check(F::decode(value.clone()).is_err(), "field.invalid")?;
    }
    Ok(())
}
