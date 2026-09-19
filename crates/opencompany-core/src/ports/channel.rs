//! The [`ChannelAdapter`] port: outbound conversation surfaces.
//!
//! Inbound messages do **not** flow through this trait (issue #1958). Ingress
//! is route-specific: operator chat arrives as `CompanyEvent::OperatorMessage`
//! via the HTTP chat route and the ACP `session/prompt` route; email/webhook
//! ingress is filed into [`crate::ports::InboxStore`] and emits
//! `CompanyEvent::WebhookReceived`; other integrations have their own paths.
//!
//! **API migration (source-breaking):** `inbound()` has been removed. The
//! deprecated shim below keeps out-of-tree implementations compiling with a
//! warning. Remove `inbound()` from any `ChannelAdapter` implementation and
//! migrate callers to the route-specific ingress paths described above.

use async_trait::async_trait;
use futures::stream::{BoxStream, StreamExt as _};

use crate::Result;
use crate::ports::types::{InboundMessage, OutboundMessage};

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

    /// **Deprecated — remove this from your implementation.**
    ///
    /// The inbound stream was removed in issue #1958. Every in-tree
    /// implementation already returned an empty stream, and nothing consumed
    /// it — inbound traffic flows through route-specific paths, not this trait.
    ///
    /// This default returns the same empty stream so existing out-of-tree
    /// implementations continue to compile with a deprecation warning instead
    /// of a hard break. Remove `inbound()` from your `ChannelAdapter` and
    /// route inbound messages through `CompanyEvent::OperatorMessage` (operator
    /// chat / ACP) or `InboxStore` (email / webhook) instead.
    #[deprecated(
        since = "0.2.4",
        note = "inbound() is gone — remove it from your ChannelAdapter impl; \
                see the ports/channel.rs module doc for the replacement paths"
    )]
    fn inbound(&self) -> BoxStream<'static, InboundMessage> {
        // Empty by design: the entire `inbound` concept was a dead end.
        // Every real implementation returned `Box::pin(futures::stream::empty())`
        // before this removal; the behaviour here is identical.
        futures::stream::empty().boxed()
    }
}
