use super::common::Directory;
use rom::operator::*;
use rom::{Action, Actor, Channel, Resource, Runtime, Storage};
use std::sync::Arc;
pub(super) type NativeCounts = Box<dyn Fn() -> [u64; 4]>;

#[derive(Clone, Resource)]
#[resource(name = "operator-notes")]
pub(super) struct Note {
    pub sent: bool,
}

pub(super) const NOTICE: Channel<String> = Channel::new("operator-notice", 1);
pub(super) const SEND: Action<Note, ()> = Action::new("send", |note, ()| {
    note.sent = true;
    Ok(vec![NOTICE.intent("PRIVATE-NOTIFICATION-PAYLOAD".into())])
});

pub(super) struct SessionOwner;
impl OperatorAuthorizer for SessionOwner {
    fn authorize(
        &self,
        actor: &Actor,
        _: OperatorAccess,
        _: Option<&WorkScope>,
        _: &mut dyn rom::AuthorizationRead,
    ) -> rom::Result<()> {
        if *actor == rom_demo::session_actor() {
            Ok(())
        } else {
            Err(rom::Error::Denied)
        }
    }
}

pub(super) fn native_store(directory: &Directory, redb: bool) -> (Arc<dyn Storage>, NativeCounts) {
    if redb {
        let store = Arc::new(rom_redb::Redb::open(directory.0.join("db")).unwrap());
        let inspect = store.clone();
        (store, Box::new(move || inspect.counts().unwrap()))
    } else {
        let store = Arc::new(rom_sqlite::Sqlite::open(directory.0.join("db")).unwrap());
        let inspect = store.clone();
        (store, Box::new(move || inspect.counts().unwrap()))
    }
}

pub(super) fn declarations() -> rom::Builder {
    Runtime::builder()
        .resource(
            Note::definition()
                .policy(|_, _, _| true)
                .allow_all_fields()
                .action(SEND),
        )
        .operator_authorizer(Arc::new(SessionOwner))
}
