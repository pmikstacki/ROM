use std::{env, fs, path::PathBuf};
fn main() {
    let mut generated = String::new();
    // Fixed accepted schema generates resource-specific helpers, not SQL.
    for (name, number) in [("InventoryQuery", "amount"), ("TicketQuery", "priority")] {
        generated.push_str(&format!(r#"
        pub struct {name}(crate::Plan);
        impl Default for {name} {{ fn default() -> Self {{ Self::new() }} }}
        impl {name} {{
            pub fn new() -> Self {{ Self(crate::Plan::default()) }}
            pub fn {number}_ge(mut self, value:u64)->Self {{ self.0.filters.push(crate::Predicate::AmountGe(value)); self }}
            pub fn title_equals(mut self, value:&str)->Self {{ self.0.filters.push(crate::Predicate::TitleEq(value.into())); self }}
            pub fn order_{number}_desc(mut self)->Self {{ self.0.order.push(crate::Order(crate::Field::Amount,true)); self }}
            pub fn order_title_asc(mut self)->Self {{ self.0.order.push(crate::Order(crate::Field::Title,false)); self }}
            pub fn finish(self)->crate::Plan {{ crate::normalize(self.0).unwrap() }}
        }}
        "#));
    }
    fs::write(
        PathBuf::from(env::var("OUT_DIR").unwrap()).join("generated.rs"),
        generated,
    )
    .unwrap();
    println!("cargo:rerun-if-changed=build.rs");
}
