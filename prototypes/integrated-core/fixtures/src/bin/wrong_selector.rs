use rom::Resource;
#[derive(Clone,Resource)]
#[resource(name="bad",crate="::rom")]
struct Example {flag:bool}
fn main() {let _=Example::flag_field().equals("wrong value");}
