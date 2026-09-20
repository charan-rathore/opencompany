//! Regression tests for issue #1871:
//! * Claim A: the `AttemptOutcome::Empty` retry arm is reachable from a single
//!   ordinary blank model completion (the commit that shipped the fix said it was
//!   not, which was wrong).
//! * Claim B: the retry called `agent.turn` a second time without first rolling
//!   back the user-turn row the failed attempt had already stamped, so the user
//!   message appeared twice in every provider request that the retry sent.
//!
//! Tests 1–3 pass against the current implementation (they pin correct
//! behaviour).  Test 4 fails against the unfixed code and passes only after
//! `run_with_steer` pops the duplicated user row before the retry.

use super::built_in_test_fixtures::*;
use super::built_in_test_fixtures_2::*;
use super::*;

/// **Claim A, reachability proof** — a single `Ok(String::new())` in the
/// scripted-provider sequence causes the wrapper to make exactly two provider
/// calls: the first triggers `EmptyProviderResponse` via `harness_turn`'s
/// `text.is_empty() && tool_calls == 0` branch and returns
/// `AttemptOutcome::Empty` at `classify_turn`, and the second attempt
/// consumes the recovery reply.
///
/// The commit that introduced `set_next_turn_overrides` reapplication
/// (PR #1766) claimed this arm was unreachable from a single blank because
/// "openhuman retries it internally" — there is no such internal retry for an
/// ordinary blank completion; that comment referred to the `#4093` re-prompt
/// which fires only when `tool_calls > 0`.  This test is the minimal proof
/// that the comment was wrong and the arm is live.
#[tokio::test]
async fn a_single_blank_script_reaches_the_empty_retry_arm() {
    let (agent, _deps) = scripted_agent(vec![Ok(String::new()), Ok("recovery".into())]);
    let (outcome, usages) = agent.run("hello").await;
    let outcome = outcome.expect("wrapper recovers");
    assert!(
        outcome.reply.contains("recovery"),
        "second attempt reply must reach the caller: {:?}",
        outcome.reply,
    );
    assert_eq!(
        usages.len(),
        2,
        "exactly two provider calls must have been made — one for the blank, one for the \
         recovery: {usages:?}",
    );
}

/// **Claim A, scope preservation** — when a chat-only turn (suppress_tools)
/// gets a blank first response and the wrapper retries, the retry must run
/// with the SAME reduced overrides (no tools, no memory agent, no active
/// goal).  Without the `set_next_turn_overrides` reapplication at the retry
/// site the second provider call would carry the agent's full 33-tool belt.
///
/// Measured via `ScriptedProvider::captured`: `(tool_count, user_count)` per
/// call.  Both entries must have `tool_count == 0` for a `suppress_tools`
/// turn.  This test fails if the reapplication line is removed.
#[tokio::test]
async fn chat_only_turn_keeps_its_reduced_scope_on_the_empty_retry() {
    let (agent, _deps, capture) =
        scripted_agent_with_capture(vec![Ok(String::new()), Ok("reply".into())]);
    // `with_chat_only_hint` is the same gate `run_with_steer` queries; it sets
    // the `CHAT_ONLY_TURN` task-local so the overrides branch fires.
    let (outcome, _usages) =
        crate::runtime::delegation::with_chat_only_hint(true, agent.run("good morning")).await;
    outcome.expect("wrapper recovers");

    let calls = capture.captured.lock().unwrap().clone();
    assert_eq!(
        calls.len(),
        2,
        "two provider calls (blank + recovery): {calls:?}"
    );
    let tool_counts: Vec<usize> = calls.iter().map(|(t, _)| *t).collect();
    assert_eq!(
        tool_counts,
        vec![0, 0],
        "both attempts must run with zero tools — the reduced chat-only scope must be \
         reapplied for the retry, not just the first attempt: {calls:?}",
    );
}

/// **Control case** — a *normal* (non-chat-only) turn that gets a blank first
/// response keeps its full tool belt on the retry.  This passes regardless of
/// the fix and ensures the assertion above cannot be satisfied by an agent
/// that has no tools at all.
#[tokio::test]
async fn a_normal_turn_keeps_its_full_toolbelt_on_the_empty_retry() {
    let (agent, _deps, capture) =
        scripted_agent_with_capture(vec![Ok(String::new()), Ok("reply".into())]);
    // No `with_chat_only_hint` → full-agentic scope.
    let (outcome, _usages) = agent.run("do the task").await;
    outcome.expect("wrapper recovers");

    let calls = capture.captured.lock().unwrap().clone();
    assert_eq!(
        calls.len(),
        2,
        "two provider calls (blank + recovery): {calls:?}"
    );
    // The scripted agent has a non-zero tool count (the roster always builds
    // a real agent with at least the baseline tools).
    let (first_tools, _) = calls[0];
    let (second_tools, _) = calls[1];
    assert_eq!(
        first_tools, second_tools,
        "tool count must be identical on both attempts for a non-chat-only turn: {calls:?}",
    );
    assert!(
        first_tools > 0,
        "a normal turn must send at least one tool — if this fails the control case is \
         vacuous and the scope-preservation test above may be passing for the wrong reason: \
         {calls:?}",
    );
}

/// **Claim B, history deduplication** — before the fix, `Agent::turn` pushed
/// the (enriched) user message to `self.history` on every call and only popped
/// the trailing empty *assistant* row on `EmptyProviderResponse`, so the first
/// attempt's user row survived into the retry.  The retry called `turn` again,
/// which appended a second copy of the same message, and the provider saw it
/// twice in the second request.
///
/// Measured via `ScriptedProvider::captured`: the `user_count` in both calls
/// must be identical.  Without the `pop_last_history_row` fix the second
/// call's count is one higher than the first's.
///
/// **This test fails against the unfixed code and passes only after the fix.**
#[tokio::test]
async fn an_empty_retry_does_not_duplicate_the_user_message_in_history() {
    let (agent, _deps, capture) =
        scripted_agent_with_capture(vec![Ok(String::new()), Ok("reply".into())]);
    let (outcome, _usages) = agent.run("hello").await;
    outcome.expect("wrapper recovers");

    let calls = capture.captured.lock().unwrap().clone();
    assert_eq!(
        calls.len(),
        2,
        "two provider calls (blank + recovery): {calls:?}"
    );
    let (_, first_user_count) = calls[0];
    let (_, second_user_count) = calls[1];
    assert_eq!(
        first_user_count, second_user_count,
        "the retry must NOT duplicate the user message — both provider requests must \
         carry the same number of user-role messages.  Before the fix the second \
         request had one extra copy (first={first_user_count}, \
         second={second_user_count}): {calls:?}",
    );
    assert!(
        first_user_count >= 1,
        "at least one user message must be present in each request: {calls:?}",
    );
}
