//! Ensure every documented curl JSON body obeys the actual generic wire contract.
#[test]
fn readme_invocations_decode_without_a_second_schema() {
    let readme = include_str!("../README.md");
    let mut invocations = 0;
    for line in readme
        .lines()
        .filter(|line| line.trim_start().starts_with("-d '"))
    {
        let json = line
            .trim()
            .strip_prefix("-d '")
            .unwrap()
            .strip_suffix('\'')
            .unwrap();
        let value: serde_json::Value = serde_json::from_str(json).unwrap();
        if value.get("operation").is_some() {
            let _: rom::Invocation = serde_json::from_value(value).unwrap();
            invocations += 1;
        }
    }
    assert_eq!(invocations, 3);
}
