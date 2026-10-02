//! Actual library observations, deliberately independent of ROM wrapper policy.
use config::{Config, File, FileFormat, Source};
use figment::{
    Figment, Profile,
    providers::{Format, Json, Serialized, Toml},
};
use rom_disposable_configuration_trials::Loader;
use serde_json::{Value, json};

#[test]
fn both_merge_objects_replace_arrays_preserve_omission_and_explicit_null() {
    for loader in [Loader::Config, Loader::Figment] {
        let actual = loader
            .merge(&[
                json!({"nested":{"a":1,"b":2},"list":["a","b"],"nullable":"old","omitted":"keep"}),
                json!({"nested":{"a":9},"list":["c"],"nullable":null}),
            ])
            .unwrap();
        assert_eq!(
            actual,
            json!({"nested":{"a":9,"b":2},"list":["c"],"nullable":null,"omitted":"keep"})
        );
        // An empty later document is no delete; rebuilding without an override reveals lower values.
        assert_eq!(
            loader.merge(&[json!({"x":1}), json!({})]).unwrap(),
            json!({"x":1})
        );
        assert_eq!(
            loader.merge(&[json!({"x":1}), json!({"x":null})]).unwrap(),
            json!({"x":null})
        );
    }
}
#[test]
fn both_mask_invalid_overridden_values_without_rom_validation() {
    for loader in [Loader::Config, Loader::Figment] {
        let v = loader
            .merge(&[json!({"enabled":"wrong"}), json!({"enabled":true})])
            .unwrap();
        assert_eq!(v["enabled"], true);
    }
}
#[test]
fn file_formats_agree_on_common_subset_and_report_parse_errors() {
    let c = Config::builder()
        .add_source(File::from_str(
            "title = 'base'\n[nested]\na = 1",
            FileFormat::Toml,
        ))
        .add_source(File::from_str(r#"{"title":"override"}"#, FileFormat::Json))
        .build()
        .unwrap();
    let f = Figment::from(Toml::string("title = 'base'\n[nested]\na = 1"))
        .merge(Json::string(r#"{"title":"override"}"#));
    assert_eq!(
        c.try_deserialize::<Value>().unwrap(),
        f.extract::<Value>().unwrap()
    );
    assert!(
        Config::builder()
            .add_source(File::from_str("{broken", FileFormat::Json))
            .build()
            .is_err()
    );
    assert!(
        Figment::from(Json::string("{broken"))
            .extract::<Value>()
            .is_err()
    );
}
#[test]
fn figment_merge_join_admerge_and_global_are_different_policies() {
    let base = Figment::from(Json::string(r#"{"x":1,"list":[1]}"#));
    assert_eq!(
        base.clone()
            .merge(Json::string(r#"{"x":2,"list":[2]}"#))
            .extract::<Value>()
            .unwrap(),
        json!({"x":2,"list":[2]})
    );
    assert_eq!(
        base.clone()
            .join(Json::string(r#"{"x":2,"list":[2]}"#))
            .extract::<Value>()
            .unwrap(),
        json!({"x":1,"list":[1]})
    );
    assert_eq!(
        base.admerge(Json::string(r#"{"list":[2]}"#))
            .extract::<Value>()
            .unwrap()["list"],
        json!([1, 2])
    );
    let f = Figment::new()
        .merge(Serialized::defaults(json!({"x":1})).profile(Profile::Global))
        .merge(Serialized::defaults(json!({"x":2})))
        .select(Profile::Default);
    assert_eq!(f.extract::<Value>().unwrap()["x"], 1);
}
#[derive(Debug, Clone)]
struct NamedSource {
    name: &'static str,
    value: config::ValueKind,
}
impl Source for NamedSource {
    fn clone_into_box(&self) -> Box<dyn Source + Send + Sync> {
        Box::new(self.clone())
    }
    fn collect(&self) -> Result<config::Map<String, config::Value>, config::ConfigError> {
        Ok(config::Map::from([(
            "enabled".into(),
            config::Value::new(Some(&self.name.to_string()), self.value.clone()),
        )]))
    }
}
#[test]
fn config_retains_winning_origin_and_contextual_error_but_raw_error_leaks_value() {
    let c = Config::builder()
        .add_source(NamedSource {
            name: "defaults:r1",
            value: true.into(),
        })
        .add_source(NamedSource {
            name: "deployment:r2",
            value: "sensitive-marker".into(),
        })
        .build()
        .unwrap();
    let raw: config::Value = c.get("enabled").unwrap();
    assert_eq!(
        raw.origin(),
        None,
        "generic get deserializes and drops origin"
    );
    let collected = c.collect().unwrap();
    assert_eq!(collected["enabled"].origin(), Some("deployment:r2"));
    let error = c.get::<bool>("enabled").unwrap_err().to_string();
    assert!(error.contains("enabled"));
    assert!(error.contains("deployment:r2"));
    assert!(error.contains("sensitive-marker"));
    println!("config error includes invalid value; ROM must map errors to safe codes");
}
#[test]
fn figment_retains_winning_metadata_and_contextual_error_but_raw_error_leaks_value() {
    let f = Figment::from(Serialized::defaults(json!({"enabled":true})))
        .merge(Json::string(r#"{"enabled":"sensitive-marker"}"#));
    let metadata = f.find_metadata("enabled").unwrap();
    assert!(metadata.name.contains("JSON"));
    let error = f.extract_inner::<bool>("enabled").unwrap_err();
    assert_eq!(error.path, vec!["enabled"]);
    assert!(error.metadata.is_some());
    assert!(error.to_string().contains("sensitive-marker"));
    println!(
        "Figment error: path and provider metadata available; raw message includes invalid value"
    );
}
#[test]
fn config_top_level_keys_are_paths_but_nested_ids_survive_literal_map_loading() {
    let c = Config::builder()
        .add_source(File::from_str(
            r#"{"alice.example[0]":"name"}"#,
            FileFormat::Json,
        ))
        .build()
        .unwrap()
        .try_deserialize::<Value>()
        .unwrap();
    let f = Figment::from(Json::string(r#"{"alice.example[0]":"name"}"#))
        .extract::<Value>()
        .unwrap();
    assert_ne!(c, f);
    assert_eq!(f["alice.example[0]"], "name");
    for loader in [Loader::Config, Loader::Figment] {
        assert_eq!(
            loader
                .merge(&[json!({"resources":{"alice.example[0]":"name"}})])
                .unwrap()["resources"]["alice.example[0]"],
            "name"
        );
    }
}
#[test]
fn actual_file_provenance_has_source_location() {
    let directory = std::env::temp_dir().join(format!("rom-config-trial-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("deployment.json");
    std::fs::write(&path, r#"{"enabled":true}"#).unwrap();
    let c = Config::builder()
        .add_source(File::from(path.clone()))
        .build()
        .unwrap();
    let value: config::Value = c.get("enabled").unwrap();
    assert_eq!(value.origin(), None);
    assert!(
        c.collect().unwrap()["enabled"]
            .origin()
            .unwrap()
            .contains("deployment.json")
    );
    let f = Figment::from(Json::file(&path));
    assert!(
        f.find_metadata("enabled")
            .unwrap()
            .source
            .as_ref()
            .unwrap()
            .to_string()
            .contains("deployment.json")
    );
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn defaults_two_files_and_named_override_have_deterministic_precedence() {
    let directory =
        std::env::temp_dir().join(format!("rom-config-precedence-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let deployment = directory.join("deployment.toml");
    let site = directory.join("site.json");
    std::fs::write(&deployment, "title = 'deployment'\nnote = 'inherited'").unwrap();
    std::fs::write(&site, r#"{"title":"site"}"#).unwrap();
    let c = Config::builder()
        .set_default("title", "default")
        .unwrap()
        .add_source(File::from(deployment.clone()))
        .add_source(File::from(site.clone()))
        .set_override("title", "operator")
        .unwrap()
        .build()
        .unwrap()
        .try_deserialize::<Value>()
        .unwrap();
    let f = Figment::from(Serialized::defaults(json!({"title":"default"})))
        .merge(Toml::file(&deployment))
        .merge(Json::file(&site))
        .merge(Serialized::defaults(json!({"title":"operator"})))
        .extract::<Value>()
        .unwrap();
    assert_eq!(c, json!({"title":"operator","note":"inherited"}));
    assert_eq!(c, f);
    std::fs::remove_dir_all(directory).unwrap();
}
