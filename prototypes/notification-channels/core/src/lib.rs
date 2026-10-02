use serde::{Serialize, de::DeserializeOwned};
use std::{collections::HashMap, future::Future, pin::Pin, time::Duration};

pub type Result<T> = std::result::Result<T, String>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeliveryId(pub String);

pub struct Delivery<P> {
    pub id: DeliveryId,
    pub payload: P,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Accepted(String),
    Retryable,
    Permanent,
    Unknown,
}

#[derive(Clone)]
pub struct Intent {
    pub id: DeliveryId,
    pub channel: String,
    pub schema: String,
    pub payload: Vec<u8>,
}

pub struct Work {
    pub intent: Intent,
    /// Durable claim generation, also the attempt number; fences stale acknowledgments.
    pub attempt: u32,
}

pub trait Outbox {
    fn claim(&mut self, now: u64, lease: u64, max_attempts: u32) -> Result<Option<Work>>;
    fn finish(
        &mut self,
        work: &Work,
        outcome: Outcome,
        next_at: u64,
        max_attempts: u32,
    ) -> Result<()>;
}

type SendFuture = Pin<Box<dyn Future<Output = Outcome> + Send>>;
type SendFn = dyn Fn(Delivery<Vec<u8>>) -> SendFuture + Send + Sync;
type ValidateFn = dyn Fn(&[u8]) -> Result<()> + Send + Sync;

struct Channel {
    schema: String,
    validate: Box<ValidateFn>,
    send: Box<SendFn>,
}

#[derive(Default)]
pub struct Channels {
    channels: HashMap<String, Channel>,
}

impl Channels {
    pub fn register<P, F, Fut>(&mut self, name: &str, schema: &str, send: F) -> Result<()>
    where
        P: DeserializeOwned + Send + 'static,
        F: Fn(Delivery<P>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Outcome> + Send + 'static,
    {
        if name.is_empty() || schema.is_empty() || name.len() > 128 || schema.len() > 128 {
            return Err("invalid channel name or schema".into());
        }
        if self.channels.contains_key(name) {
            return Err("duplicate channel".into());
        }
        self.channels.insert(
            name.into(),
            Channel {
                schema: schema.into(),
                validate: Box::new(|bytes| {
                    serde_json::from_slice::<P>(bytes)
                        .map(|_| ())
                        .map_err(|_| "invalid payload".into())
                }),
                send: Box::new(move |delivery| {
                    match serde_json::from_slice::<P>(&delivery.payload) {
                        Ok(payload) => Box::pin(send(Delivery {
                            id: delivery.id,
                            payload,
                        })),
                        Err(_) => Box::pin(async { Outcome::Permanent }),
                    }
                }),
            },
        );
        Ok(())
    }

    pub fn intent<P: Serialize>(
        &self,
        id: &str,
        channel: &str,
        schema: &str,
        payload: &P,
    ) -> Result<Intent> {
        if id.is_empty() || id.len() > 128 {
            return Err("invalid delivery id".into());
        }
        let registration = self.channels.get(channel).ok_or("unknown channel")?;
        if registration.schema != schema {
            return Err("schema mismatch".into());
        }
        let payload = serde_json::to_vec(payload).map_err(|_| "payload encoding failed")?;
        if payload.len() > 65536 {
            return Err("payload too large".into());
        }
        (registration.validate)(&payload)?;
        Ok(Intent {
            id: DeliveryId(id.into()),
            channel: channel.into(),
            schema: schema.into(),
            payload,
        })
    }

    async fn send(&self, intent: &Intent) -> Outcome {
        let Some(channel) = self.channels.get(&intent.channel) else {
            return Outcome::Permanent;
        };
        if channel.schema != intent.schema || intent.payload.len() > 65536 {
            return Outcome::Permanent;
        }
        (channel.send)(Delivery {
            id: intent.id.clone(),
            payload: intent.payload.clone(),
        })
        .await
    }
}

pub struct Dispatcher {
    pub channels: Channels,
    pub max_attempts: u32,
    pub base_backoff: u64,
    pub lease: u64,
    pub timeout: Duration,
}

impl Dispatcher {
    /// One bounded attempt. `now` is an explicit durable-clock tick (seconds).
    /// No database transaction is held while awaiting the registered function.
    pub async fn step(&self, outbox: &mut impl Outbox, now: u64) -> Result<bool> {
        if self.max_attempts == 0
            || self.max_attempts > 16
            || self.base_backoff == 0
            || self.base_backoff > 86400
            || self.timeout.is_zero()
            || self.timeout >= Duration::from_secs(self.lease)
        {
            return Err("invalid worker limits".into());
        }
        let Some(work) = outbox.claim(now, self.lease, self.max_attempts)? else {
            return Ok(false);
        };
        let outcome = tokio::time::timeout(self.timeout, self.channels.send(&work.intent))
            .await
            .unwrap_or(Outcome::Unknown);
        let delay = self
            .base_backoff
            .saturating_mul(1u64 << work.attempt.saturating_sub(1).min(15))
            .min(86400);
        let next_at = now.checked_add(delay).ok_or("clock overflow")?;
        outbox.finish(&work, outcome, next_at, self.max_attempts)?;
        Ok(true)
    }
}
