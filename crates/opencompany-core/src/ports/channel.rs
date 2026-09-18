//! The [`ChannelAdapter`] port: outbound conversation surfaces.
//!
//! Inbound messages do not flow through this trait. They arrive as
//! `CompanyEvent::OperatorMessage` through the HTTP chat route and the ACP
//! `session/prompt` route. The trait is an outbound-only sink over the event
//! log (issue #1958).

use async_trait::async_trait;

use crate::Result;
use crate::ports::types::OutboundMessage;

/// A conversation surface. The built-in `"operator"` channel is always
/// present; others (email, tinyplace-dm, …) usually delegate to OpenHuman.
///
/// Outbound-only: there is no inbound stream. Operator and ACP messages
/// enter through the event log, not this port.
#[async_trait]
pub trait ChannelAdapter: Send + Sync {
    /// The channel's stable id, e.g. `"operator"` or `"email"`.
    fn channel_id(&self) -> &str;
    /// Sends an outbound message on this channel.
    async fn send(&self, msg: OutboundMessage) -> Result<()>;
}
