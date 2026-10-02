//! Application code: no repository, HTTP controller, broadcaster or query worker.
use rom_library_slice::{
    json, resource, Access, Actor, Authorizer, Command, Query, Resource, Runtime, Value,
};
fn complete(data: &mut Value, _input: &Value) -> Result<(), String> {
    data["done"] = json!(true);
    Ok(())
}
resource! {Task("tasks"){title:String,done:bool} actions{"complete"=>complete}}
struct HostPolicy;
impl Authorizer for HostPolicy {
    fn allows(&self, actor: &Actor, _: Access<'_>, _: Option<&Resource>) -> bool {
        actor.0 == "demo"
    }
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = Runtime::builder()
        .resource(Task::definition())
        .authorizer(HostPolicy)
        .open_sqlite(":memory:")
        .await?;
    let actor = Actor::new("demo");
    let mut open_tasks = runtime
        .live(&actor, Query::equals(Task::KIND, "done", false))
        .await?;
    runtime
        .execute(
            &actor,
            Command::create(Task::KIND, "one", json!({"title":"Try ROM","done":false})),
        )
        .await?;
    println!("open tasks: {}", open_tasks.changed().await?.rows.len());
    runtime
        .execute(
            &actor,
            Command::action(Task::KIND, "one", "complete", 1, json!({})),
        )
        .await?;
    println!("after complete: {}", open_tasks.changed().await?.rows.len());
    runtime.shutdown().await?;
    Ok(())
}
