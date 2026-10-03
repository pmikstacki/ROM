//! Test fixtures for the actual state shape before native format 8.
pub fn state(mut value: serde_json::Value) -> String {
    value.as_object_mut().unwrap().remove("operator");
    for record in value["work"]["work"].as_object_mut().unwrap().values_mut() {
        record.as_object_mut().unwrap().remove("revision");
        record["pending"]
            .as_object_mut()
            .unwrap()
            .remove("delivery_profile");
    }
    serde_json::to_string(&value).unwrap()
}
