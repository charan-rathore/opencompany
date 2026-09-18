//! The [`ChannelAdapter`] port: outbound conversation surfaces.
//!
//! Inbound messages do **not** flow through this trait (issue #1958). Ingress
//! is route-specific: operator chat arrives as `CompanyEvent::OperatorMessage`
//! via the HTTP chat route and the ACP `session/prompt` route; email/webhook
//! ingress is filed into [`crate::ports::InboxStore`] and emits
//! `CompanyEvent::WebhookReceived`; other integrations have their own paths.
//!
//! **API migration:** the removal of `inbound()` is a source-compatibility
//! break for downstream `ChannelAdapter` implementers. There is no replacement.
//! Remove `inbound()` from any out-of-tree implementation of this trait.

use async_trait::async_trait;

use crate::Result;
use crate::ports::types::OutboundMessage;

/// A conversation surface. The built-in `"operator"` channel is always
/// present; others (email, tinyplace-dm, …) usually delegate to OpenHuman.
///
/// Outbound-only: there is no inbound stream. Inbound messages reach the
/// runtime through route-specific paths, not through this trait.
#[async_trait]
pub trait ChannelAdapter: Send + Sync {
    /// The channel's stable id, e.g. `"operator"` or `"email"`.
    fn channel_id(&self) -> &str;
    /// Sends an outbound message on this channel.
    async fn send(&self, msg: OutboundMessage) -> Result<()>;
}
