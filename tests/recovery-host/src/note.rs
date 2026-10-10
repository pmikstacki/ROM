use rom::{Access, Action, Actor, Input, Intent, PrincipalKind, Resource, Result, json};

#[derive(Clone, Resource)]
#[resource(name = "recovery-notes")]
pub struct Note {
    pub owner: String,
    pub title: String,
    pub count: u64,
    pub optional: Option<String>,
    pub flag: bool,
}

#[derive(Clone, Input)]
pub struct Save {
    pub title: String,
    pub count: u64,
}

fn save(note: &mut Note, input: Save) -> Result<Vec<Intent>> {
    note.title = input.title;
    note.count = input.count;
    Ok(vec![Intent::new(
        "recovery-note-saved",
        json!({"title": note.title}),
    )])
}

pub fn declarations() -> rom::Builder {
    rom::Runtime::builder().resource(
        Note::definition()
            .allow_all_fields()
            .discovery_policy(|actor: &Actor, _| {
                actor.authority == "recovery-fixture"
                    && actor.principal_kind() == PrincipalKind::Human
            })
            .policy(|actor: &Actor, _: Access, note: &Note| {
                actor.authority == "recovery-fixture" && actor.subject == note.owner
            })
            .action(Action::new("save", save)),
    )
}
