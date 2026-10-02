use rom::Resource;
#[derive(Clone,Resource)]
#[resource(name="renamed",crate="::rom")]
struct Example {flag:bool}
fn main() {let value=Example{flag:false};assert_eq!(Example::decode(value.encode()).unwrap().flag,false);assert_eq!(Example::descriptor().fields.len(),1);}
