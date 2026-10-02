use rom::{Action,Command,Intent,Resource,Result};
#[derive(Clone,Resource)]
#[resource(name="bad",crate="::rom")]
struct Example {flag:bool}
fn set(r:&mut Example,value:bool)->Result<Vec<Intent>>{r.flag=value;Ok(vec![])}
fn main() {let action=Action::new("set",set);let _=Command::action("one",action,"wrong input");}
