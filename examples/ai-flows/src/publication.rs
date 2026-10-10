//! Application-owned immutable source capture, preparation and separate head publication.
use crate::owned::{self, Owned};
use rom::{Action, Resource, ResourceRef};
use rom_ai::{AiError, AiResult, ToolRegistry};
#[derive(Clone, Resource)]
#[resource(name = "consumer_article_drafts", version = 1)]
pub struct Draft {
    pub owner: String,
    pub text: String,
}
#[derive(Clone, Resource)]
#[resource(name = "consumer_article_editions", version = 1)]
pub struct Edition {
    pub owner: String,
    pub draft: ResourceRef<Draft>,
    pub draft_revision: u64,
    pub text: String,
    pub prepared: bool,
}
#[derive(Clone, Resource)]
#[resource(name = "consumer_article_heads", version = 1)]
pub struct Head {
    pub owner: String,
    pub edition: Option<ResourceRef<Edition>>,
}
#[derive(Clone, rom::Input)]
pub struct PrepareEdition {
    pub draft_revision: u64,
    pub text: String,
}
#[derive(Clone, rom::Input)]
struct DraftSource {
    draft_revision: u64,
    text: String,
}
pub const PREPARE_EDITION: Action<Edition, PrepareEdition> =
    Action::new("prepare_edition", |edition, input| {
        if edition.prepared
            || input.draft_revision != edition.draft_revision
            || input.text != edition.text
        {
            return Err(rom::Error::Denied);
        }
        edition.prepared = true;
        Ok(vec![])
    });
pub const PUBLISH_HEAD: Action<Head, ResourceRef<Edition>> =
    Action::new("publish_head", |head, edition| {
        head.edition = Some(edition);
        Ok(vec![])
    });
impl Owned for Draft {
    fn owner(&self) -> &str {
        &self.owner
    }
    fn validate(&self) -> rom::Result<()> {
        bounded_source(&self.text)
    }
    fn transition(_: Option<&Self>, _: Option<&Self>) -> rom::Result<()> {
        Ok(())
    }
}
impl Owned for Edition {
    fn owner(&self) -> &str {
        &self.owner
    }
    fn validate(&self) -> rom::Result<()> {
        if self.draft_revision == 0 {
            return Err(rom::Error::Denied);
        }
        bounded_source(&self.text)
    }
    fn transition(before: Option<&Self>, after: Option<&Self>) -> rom::Result<()> {
        if let Some(before) = before {
            let after = after.ok_or(rom::Error::Denied)?;
            if after.draft != before.draft
                || after.draft_revision != before.draft_revision
                || after.text != before.text
                || (before.prepared && !after.prepared)
            {
                return Err(rom::Error::Denied);
            }
        } else if after.is_some_and(|edition| edition.prepared) {
            return Err(rom::Error::Denied);
        }
        Ok(())
    }
}
impl Owned for Head {
    fn owner(&self) -> &str {
        &self.owner
    }
    fn validate(&self) -> rom::Result<()> {
        Ok(())
    }
    fn transition(_: Option<&Self>, _: Option<&Self>) -> rom::Result<()> {
        Ok(())
    }
}
fn bounded_source(text: &str) -> rom::Result<()> {
    if text.is_empty() || text.len() > 4096 {
        Err(rom::Error::TooLarge)
    } else {
        Ok(())
    }
}
pub fn register(builder: rom::Builder) -> rom::Builder {
    builder
        .resource(owned::definition::<Draft>())
        .resource(owned::definition::<Edition>().action(PREPARE_EDITION))
        .resource(owned::definition::<Head>().action(PUBLISH_HEAD))
}
pub fn tools(version: u32) -> AiResult<ToolRegistry> {
    let mut registry = ToolRegistry::new(version)?;
    registry.read::<String, DraftSource, _>(
        "read_draft",
        "Read the current authorized draft revision",
        |context, id| {
            Box::pin(async move {
                let snapshot = context
                    .read::<Draft>(&id)
                    .await
                    .map_err(|_| AiError::Denied)?;
                let draft = snapshot.value.ok_or(AiError::Denied)?;
                Ok(DraftSource {
                    draft_revision: snapshot.revision,
                    text: draft.text,
                })
            })
        },
    )?;
    registry.action(
        "prepare_edition",
        "Prepare the application's captured immutable edition",
        PREPARE_EDITION,
    )?;
    registry.action(
        "publish_head",
        "Publish an already prepared authorized edition",
        PUBLISH_HEAD,
    )?;
    Ok(registry)
}
