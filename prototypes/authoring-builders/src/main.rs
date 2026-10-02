fn main() {
    #[cfg(feature = "bon")]
    {
        use authoring_builders::bon_style::{retry_policy, ResourceConfig};
        authoring_builders::bon_style::verify();
        for production in [false, true] {
            let config =
                retry_policy(ResourceConfig::builder().name("external"), production).build();
            assert_eq!(config.retries, if production { 5 } else { 3 });
        }
        println!("PASS bon: defaults, external reusable helper, conditional optional value, absent/null/value");
    }
    #[cfg(feature = "typed")]
    {
        use authoring_builders::typed::{retry_policy, ResourceConfig};
        authoring_builders::typed::verify();
        authoring_builders::typed::verify_optional_fallback();
        for production in [false, true] {
            let config =
                retry_policy(ResourceConfig::builder().name("external"), production).build();
            assert_eq!(config.retries, if production { 5 } else { 3 });
        }
        println!("PASS typed-builder: defaults, external reusable helper, conditional optional value, absent/null/value");
    }
    #[cfg(feature = "manual")]
    {
        authoring_builders::manual::verify();
        println!("PASS handwritten: defaults, reusable helper, conditional reassignment, absent/null/value");
    }
}
