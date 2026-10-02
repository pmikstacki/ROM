#![allow(dead_code)]
use rom::Resource;
#[derive(Resource)]
#[resource(name = "renamed", crate = "::rom")]
struct Renamed {
    active: bool,
    notes: Vec<Option<String>>,
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_crate_path_survives_cargo_dependency_rename() {
        assert_eq!(Renamed::descriptor().name, "renamed");
        assert!(Renamed {
            active: false,
            notes: vec![None]
        }
        .validate()
        .is_ok());
    }
}
