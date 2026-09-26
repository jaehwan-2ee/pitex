//! `LiveCompileScheduler` — the pure, deterministic state machine behind
//! live auto compile. No threads, timers or sleeps: the caller owns the
//! clock and feeds millisecond timestamps in. Every event returns the
//! [`LiveRequest`]s the caller must perform (start/cancel), nothing else
//! happens inside.
//!
//! Integration contract:
//!
//! - `note_edit(now)` on every real source mutation (also while IME
//!   composition is in progress — the deadline just keeps moving).
//! - `poll(now, composing)` when the caller's debounce timer fires (arm it
//!   for [`pending_deadline`]) and whenever work may have been freed (it
//!   is idempotent).
//! - `request_manual()` when the user starts a manual build.
//! - `completed(token, now, composing)` exactly once for every started
//!   run — including runs that never launched a process — so the slot is
//!   released and queued work can dispatch.
//! - `invalidate()` on workspace switch/close or build-target/command
//!   changes; `set_enabled(false)` on the toggle. Both retire only live
//!   work — an active manual run keeps ownership.
//!
//! Stale-result suppression: a live completion is reported
//! [`LiveCompletion::Superseded`] when newer edits were recorded, an
//!   invalidation bumped the generation, or the run was asked to cancel —
//!   the UI clears its busy state but must not publish the artifact,
//!   log, issues or SyncTeX binding. A token that is not the active run
//!   is [`LiveCompletion::Stale`] and changes nothing.

use std::fmt;

/// Default idle debounce (matches `liveCompileDelayMilliseconds`).
pub const DEFAULT_DELAY_MS: u64 = 700;
/// Settings clamp bounds — callers clamp before `set_delay`.
pub const MIN_DELAY_MS: u64 = 200;
pub const MAX_DELAY_MS: u64 = 10_000;
/// Remote workspaces never debounce shorter than this — the upload and
/// device round-trip dominate anything smaller.
pub const REMOTE_MIN_DELAY_MS: u64 = 1_500;

/// The two owners of the single build slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LiveRunKind {
    Live,
    Manual,
}

/// Identity of one started run; completions echo it back verbatim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LiveRunToken {
    /// Unique per scheduler — ordering is never inferred from it.
    pub id: u64,
    pub kind: LiveRunKind,
    /// `invalidate()` generation at start; a lower one is obsolete.
    pub generation: u64,
}
impl fmt::Display for LiveRunToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind = match self.kind {
            LiveRunKind::Live => "live",
            LiveRunKind::Manual => "manual",
        };
        write!(f, "{kind}-{}-g{}", self.id, self.generation)
    }
}

/// What the scheduler asks the caller to perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveRequest {
    /// Start a live build covering the newest pending edit.
    /// `deadline_ms` echoes the debounce deadline that fired.
    StartLive { token: LiveRunToken, deadline_ms: u64 },
    /// Start the requested manual build.
    StartManual { token: LiveRunToken },
    /// Cancel the in-flight run — always a live one; the scheduler never
    /// cancels manual work.
    Cancel { token: LiveRunToken },
}

/// How `completed` classified the finishing run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveCompletion {
    /// The finishing run is current — publish its result.
    Current(LiveRunToken),
    /// The run finished but was superseded (newer edits, invalidation or
    /// a manual request): clear busy state, suppress its artifacts.
    Superseded(LiveRunToken),
    /// Not the active run — an obsolete or unknown completion. Nothing
    /// changed; the newer active run keeps its slot.
    Stale,
}

#[derive(Debug, Clone, Copy)]
struct PendingEdit {
    /// Millisecond timestamp of the newest edit (deadline base, so a
    /// later `set_delay` re-derives from the edit, not from "now").
    edited_at_ms: u64,
    deadline_ms: u64,
    generation: u64,
}

#[derive(Debug, Clone, Copy)]
struct ActiveRun {
    token: LiveRunToken,
    /// A cancel request was already emitted — never emit another.
    cancel_requested: bool,
}

#[derive(Debug, Clone)]
pub struct LiveCompileScheduler {
    enabled: bool,
    delay_ms: u64,
    /// Bumped by `invalidate()`/toggle-off; older-generation work is dead.
    generation: u64,
    next_id: u64,
    pending: Option<PendingEdit>,
    active: Option<ActiveRun>,
    /// A manual build asked for while a live run still holds the slot.
    queued_manual: bool,
}

impl Default for LiveCompileScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl LiveCompileScheduler {
    pub fn new() -> Self {
        Self {
            enabled: false,
            delay_ms: DEFAULT_DELAY_MS,
            generation: 0,
            next_id: 0,
            pending: None,
            active: None,
            queued_manual: false,
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }
    pub fn delay_ms(&self) -> u64 {
        self.delay_ms
    }
    /// Deadline the caller's timer should fire at, if an edit is pending.
    pub fn pending_deadline(&self) -> Option<u64> {
        self.pending.map(|p| p.deadline_ms)
    }
    /// The run currently holding the slot.
    pub fn active(&self) -> Option<LiveRunToken> {
        self.active.map(|a| a.token)
    }
    /// True while a live build owns the slot (also when cancelling).
    pub fn live_active(&self) -> bool {
        self.active
            .map(|a| a.token.kind == LiveRunKind::Live)
            .unwrap_or(false)
    }
    /// True while a manual build owns the slot.
    pub fn manual_active(&self) -> bool {
        self.active
            .map(|a| a.token.kind == LiveRunKind::Manual)
            .unwrap_or(false)
    }
    /// Edits or a queued manual wait behind the active run.
    pub fn has_pending_work(&self) -> bool {
        self.pending.is_some() || self.queued_manual
    }

    /// May events/results stamped with `token` be applied to the UI now?
    /// True only for the active run, and for a live run only while it is
    /// still current — once an edit, manual request or invalidation asked
    /// it to cancel, its streaming log/issue/binding events are stale and
    /// must be dropped (busy state still clears at `completed`).
    /// A manual run's results are always accepted while it is the active
    /// run — including after a toggle-off/invalidate, since a manual
    /// completion is still `Current`; the caller's workspace guard
    /// handles results landing in the wrong window.
    pub fn accepts_active_result(&self, token: LiveRunToken) -> bool {
        let Some(active) = &self.active else { return false };
        if active.token != token {
            return false;
        }
        match token.kind {
            LiveRunKind::Manual => true,
            LiveRunKind::Live => {
                !active.cancel_requested && token.generation == self.generation
            }
        }
    }

    /// Toggle. Off retires live work: pending edits drop, the generation
    /// moves and a live run is asked to cancel. Unlike `invalidate` it is
    /// not a context switch — an explicitly queued manual request and a
    /// running manual build both keep their place. On does not schedule
    /// anything; the next edit does.
    pub fn set_enabled(&mut self, enabled: bool) -> Vec<LiveRequest> {
        if self.enabled == enabled {
            return Vec::new();
        }
        self.enabled = enabled;
        if enabled {
            Vec::new()
        } else {
            self.generation += 1;
            self.pending = None;
            let mut requests = Vec::new();
            self.cancel_active_live(&mut requests);
            requests
        }
    }

    /// New debounce delay (already clamped by the caller). A pending edit
    /// re-derives its deadline from its edit timestamp, so shortening the
    /// delay fires earlier rather than sliding a new full delay.
    pub fn set_delay(&mut self, delay_ms: u64) {
        self.delay_ms = delay_ms;
        if let Some(p) = &mut self.pending {
            p.deadline_ms = p.edited_at_ms.saturating_add(delay_ms);
        }
    }

    /// Workspace switch/close or target/command change: retire all work
    /// belonging to the old context. Pending edits and a queued manual
    /// are dropped; a live run is asked to cancel; an active manual run
    /// keeps ownership (its completion stays `Current` — the caller's
    /// workspace guard decides whether it still displays).
    pub fn invalidate(&mut self) -> Vec<LiveRequest> {
        self.generation += 1;
        self.pending = None;
        self.queued_manual = false;
        let mut requests = Vec::new();
        self.cancel_active_live(&mut requests);
        requests
    }

    /// A real source edit: record the newest deadline and cancel a
    /// superseded live run immediately. During a manual run the edit just
    /// stays pending — manual builds keep priority.
    pub fn note_edit(&mut self, now_ms: u64) -> Vec<LiveRequest> {
        if !self.enabled {
            return Vec::new();
        }
        self.pending = Some(PendingEdit {
            edited_at_ms: now_ms,
            deadline_ms: now_ms.saturating_add(self.delay_ms),
            generation: self.generation,
        });
        let mut requests = Vec::new();
        self.cancel_active_live(&mut requests);
        requests
    }

    /// The user asked for a manual build. Manual work covers the current
    /// source, so any pending live edit is consumed; a running live build
    /// is cancelled and the manual run queues behind it — never overlaps.
    pub fn request_manual(&mut self) -> Vec<LiveRequest> {
        self.pending = None;
        self.queued_manual = true;
        let mut requests = Vec::new();
        self.cancel_active_live(&mut requests);
        self.dispatch_manual(&mut requests);
        requests
    }

    /// Timer poll: emit the pending live start once its deadline passed,
    /// the slot is free and no IME composition is in progress. Emits at
    /// most one start per poll.
    pub fn poll(&mut self, now_ms: u64, composing: bool) -> Vec<LiveRequest> {
        let mut requests = Vec::new();
        self.dispatch(now_ms, composing, &mut requests);
        requests
    }

    /// A started run finished (any lifecycle, including never-launched).
    /// Frees the slot and dispatches whatever queued behind it — a queued
    /// manual first, then a live edit whose deadline already elapsed (no
    /// second debounce wait after a cancelled run).
    pub fn completed(
        &mut self,
        token: LiveRunToken,
        now_ms: u64,
        composing: bool,
    ) -> (LiveCompletion, Vec<LiveRequest>) {
        let status = match &self.active {
            Some(active) if active.token == token => {
                let superseded = match token.kind {
                    LiveRunKind::Live => {
                        active.cancel_requested
                            || self.pending.is_some()
                            || token.generation != self.generation
                    }
                    // A manual result may display even with newer input
                    // pending — the pending live run then builds latest.
                    LiveRunKind::Manual => false,
                };
                self.active = None;
                if superseded {
                    LiveCompletion::Superseded(token)
                } else {
                    LiveCompletion::Current(token)
                }
            }
            // An obsolete completion changes nothing — in particular it
            // must not dispatch a due pending edit (the caller's timer
            // owns that).
            _ => return (LiveCompletion::Stale, Vec::new()),
        };
        let mut requests = Vec::new();
        self.dispatch(now_ms, composing, &mut requests);
        (status, requests)
    }

    fn mint(&mut self, kind: LiveRunKind) -> LiveRunToken {
        self.next_id += 1;
        LiveRunToken {
            id: self.next_id,
            kind,
            generation: self.generation,
        }
    }

    fn cancel_active_live(&mut self, requests: &mut Vec<LiveRequest>) {
        if let Some(active) = &mut self.active {
            if active.token.kind == LiveRunKind::Live && !active.cancel_requested {
                active.cancel_requested = true;
                requests.push(LiveRequest::Cancel {
                    token: active.token,
                });
            }
        }
    }

    fn dispatch_manual(&mut self, requests: &mut Vec<LiveRequest>) {
        if self.active.is_some() || !self.queued_manual {
            return;
        }
        self.queued_manual = false;
        let token = self.mint(LiveRunKind::Manual);
        self.active = Some(ActiveRun {
            token,
            cancel_requested: false,
        });
        requests.push(LiveRequest::StartManual { token });
    }

    fn dispatch(&mut self, now_ms: u64, composing: bool, requests: &mut Vec<LiveRequest>) {
        if self.active.is_some() {
            return;
        }
        if self.queued_manual {
            self.dispatch_manual(requests);
            return;
        }
        let Some(pending) = self.pending else { return };
        if !self.enabled
            || composing
            || now_ms < pending.deadline_ms
            || pending.generation != self.generation
        {
            return;
        }
        self.pending = None;
        let token = self.mint(LiveRunKind::Live);
        self.active = Some(ActiveRun {
            token,
            cancel_requested: false,
        });
        requests.push(LiveRequest::StartLive {
            token,
            deadline_ms: pending.deadline_ms,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enabled() -> LiveCompileScheduler {
        let mut s = LiveCompileScheduler::new();
        s.set_enabled(true);
        s
    }
    fn live_start(requests: &[LiveRequest]) -> LiveRunToken {
        match requests {
            [LiveRequest::StartLive { token, .. }] => *token,
            other => panic!("expected one StartLive, got {other:?}"),
        }
    }

    #[test]
    fn disabled_ignores_edits_and_poll() {
        let mut s = LiveCompileScheduler::new();
        assert!(s.note_edit(0).is_empty());
        assert!(s.poll(10_000, false).is_empty());
        assert_eq!(s.pending_deadline(), None);
    }

    #[test]
    fn rapid_edits_coalesce_to_one_deadline() {
        let mut s = enabled();
        assert!(s.note_edit(0).is_empty());
        assert!(s.note_edit(300).is_empty());
        assert!(s.note_edit(600).is_empty());
        assert_eq!(s.pending_deadline(), Some(1_300));
        assert!(s.poll(1_299, false).is_empty());
        let token = live_start(&s.poll(1_300, false));
        assert_eq!(s.active(), Some(token));
        // The started run consumed the pending edit — no second start.
        assert!(s.poll(5_000, false).is_empty());
    }

    #[test]
    fn composing_holds_the_deadline_until_lifted() {
        let mut s = enabled();
        s.note_edit(0);
        assert!(s.poll(700, true).is_empty());
        let token = live_start(&s.poll(700, false));
        assert!(s.live_active());
        assert_eq!(s.active(), Some(token));
    }

    #[test]
    fn edit_cancels_live_but_not_manual() {
        let mut s = enabled();
        s.note_edit(0);
        let live = live_start(&s.poll(700, false));
        let requests = s.note_edit(800);
        assert_eq!(requests, [LiveRequest::Cancel { token: live }]);
        // A second edit does not re-cancel.
        assert!(s.note_edit(810).is_empty());
        // Manual active: an edit just stays pending.
        let mut s = enabled();
        let manual = match s.request_manual().as_slice() {
            [LiveRequest::StartManual { token }] => *token,
            other => panic!("expected StartManual, got {other:?}"),
        };
        assert!(s.note_edit(0).is_empty());
        assert_eq!(s.active(), Some(manual));
        assert_eq!(s.pending_deadline(), Some(700));
    }

    #[test]
    fn cancelled_live_restarts_without_a_second_delay() {
        let mut s = enabled();
        s.note_edit(0);
        let first = live_start(&s.poll(700, false));
        s.note_edit(800); // cancels `first`, deadline 1500
        // Cancellation still in flight — pending is kept, nothing starts.
        assert!(s.poll(1_500, false).is_empty());
        // The old run finishes after the new deadline: dispatch immediately.
        let (status, requests) = s.completed(first, 1_700, false);
        assert_eq!(status, LiveCompletion::Superseded(first));
        let second = live_start(&requests);
        assert_ne!(first, second);
        let (status, _) = s.completed(second, 2_000, false);
        assert_eq!(status, LiveCompletion::Current(second));
    }

    #[test]
    fn superseded_live_never_publishes() {
        let mut s = enabled();
        s.note_edit(0);
        let live = live_start(&s.poll(700, false));
        // An edit raced the in-flight run; it succeeds anyway — the
        // completion is superseded so the stale PDF is never published.
        s.note_edit(900);
        let (status, requests) = s.completed(live, 1_000, false);
        assert_eq!(status, LiveCompletion::Superseded(live));
        // Deadline (1600) not yet reached — poll later.
        assert!(requests.is_empty());
        let next = live_start(&s.poll(1_600, false));
        let (status, _) = s.completed(next, 2_000, false);
        assert_eq!(status, LiveCompletion::Current(next));
    }

    #[test]
    fn manual_takes_priority_and_pending_edits_follow() {
        let mut s = enabled();
        s.note_edit(0);
        let live = live_start(&s.poll(700, false));
        let requests = s.request_manual();
        // Pending edit consumed by the manual request; the live run is
        // cancelled but the manual run must NOT start while it still
        // holds the slot — builds never overlap.
        assert_eq!(s.pending_deadline(), None);
        assert_eq!(requests, [LiveRequest::Cancel { token: live }]);
        assert_eq!(s.active(), Some(live));
        // The cancelled live run finishes: superseded, manual dispatches.
        let (status, requests) = s.completed(live, 1_000, false);
        assert_eq!(status, LiveCompletion::Superseded(live));
        let manual = match requests.as_slice() {
            [LiveRequest::StartManual { token }] => *token,
            other => panic!("expected StartManual, got {other:?}"),
        };
        assert!(s.manual_active());
        // Edits during the manual run stay pending; the manual result
        // still publishes, then the newest source compiles immediately.
        s.note_edit(1_100);
        let (status, requests) = s.completed(manual, 2_000, false);
        assert_eq!(status, LiveCompletion::Current(manual));
        let next = live_start(&requests);
        let (status, _) = s.completed(next, 2_500, false);
        assert_eq!(status, LiveCompletion::Current(next));
    }

    #[test]
    fn stale_completion_cannot_finish_the_newer_run() {
        let mut s = enabled();
        s.note_edit(0);
        let first = live_start(&s.poll(700, false));
        s.note_edit(800);
        let (_, requests) = s.completed(first, 1_600, false);
        let second = live_start(&requests);
        // A duplicate completion for the old run changes nothing.
        let (status, requests) = s.completed(first, 1_700, false);
        assert_eq!(status, LiveCompletion::Stale);
        assert!(requests.is_empty());
        assert_eq!(s.active(), Some(second));
    }

    #[test]
    fn stale_completion_never_dispatches_pending_work() {
        // No run is active but an edit's deadline has passed; a stray
        // completion echoing an unknown token must not launch the build —
        // only the caller's timer (poll) may.
        let mut s = enabled();
        s.note_edit(0);
        let stray = LiveRunToken { id: 99, kind: LiveRunKind::Live, generation: 0 };
        let (status, requests) = s.completed(stray, 10_000, false);
        assert_eq!(status, LiveCompletion::Stale);
        assert!(requests.is_empty());
        assert_eq!(s.pending_deadline(), Some(700));
        assert!(s.active().is_none());
    }

    #[test]
    fn accepts_active_result_guards_streaming_events() {
        let mut s = enabled();
        s.note_edit(0);
        let live = live_start(&s.poll(700, false));
        assert!(s.accepts_active_result(live));
        // Superseded by a newer edit — its late log/issue events drop.
        s.note_edit(800);
        assert!(!s.accepts_active_result(live));
        // Cancel in flight; it finishes superseded, nothing due yet.
        let (status, requests) = s.completed(live, 1_000, false);
        assert_eq!(status, LiveCompletion::Superseded(live));
        assert!(requests.is_empty());
        // The next live run accepts again.
        let next = live_start(&s.poll(1_500, false));
        assert!(s.accepts_active_result(next));
        // A manual request supersedes it; the manual token accepts while
        // active even with pending live work queued behind it.
        assert_eq!(s.request_manual(), [LiveRequest::Cancel { token: next }]);
        assert!(!s.accepts_active_result(next));
        let (_, requests) = s.completed(next, 1_600, false);
        let manual = match requests.as_slice() {
            [LiveRequest::StartManual { token }] => *token,
            other => panic!("expected StartManual, got {other:?}"),
        };
        s.note_edit(1_700);
        assert!(s.accepts_active_result(manual));
        // Unknown/wrong-identity tokens never accept.
        let stray = LiveRunToken { id: 77, kind: LiveRunKind::Manual, generation: 0 };
        assert!(!s.accepts_active_result(stray));
    }

    #[test]
    fn disabling_during_manual_still_accepts_its_events() {
        // A manual build keeps publishing streamed log/issue events after
        // live compile is switched off — only live work is retired.
        let mut s = enabled();
        let manual = match s.request_manual().as_slice() {
            [LiveRequest::StartManual { token }] => *token,
            other => panic!("expected StartManual, got {other:?}"),
        };
        assert!(s.set_enabled(false).is_empty());
        assert!(s.accepts_active_result(manual));
        let (status, _) = s.completed(manual, 5_000, false);
        assert_eq!(status, LiveCompletion::Current(manual));
    }

    #[test]
    fn queued_manual_survives_toggle_off() {
        // Live active → user requests a manual build → toggles live off.
        // The manual request stays queued behind the live cancellation
        // and starts when that run completes.
        let mut s = enabled();
        s.note_edit(0);
        let live = live_start(&s.poll(700, false));
        assert_eq!(s.request_manual(), [LiveRequest::Cancel { token: live }]);
        let requests = s.set_enabled(false);
        assert!(requests.is_empty()); // live cancel already requested
        assert!(s.has_pending_work());
        let (status, requests) = s.completed(live, 1_000, false);
        assert_eq!(status, LiveCompletion::Superseded(live));
        match requests.as_slice() {
            [LiveRequest::StartManual { .. }] => assert!(s.manual_active()),
            other => panic!("expected StartManual, got {other:?}"),
        }
    }

    #[test]
    fn invalidate_drops_queued_manual() {
        // Context switch — the queued manual belonged to the old project.
        let mut s = enabled();
        s.note_edit(0);
        let live = live_start(&s.poll(700, false));
        s.request_manual();
        s.invalidate();
        let (_, requests) = s.completed(live, 1_000, false);
        assert!(requests.is_empty());
    }

    #[test]
    fn toggle_off_invalidates_and_cancels_only_live() {
        let mut s = enabled();
        s.note_edit(0);
        let live = live_start(&s.poll(700, false));
        let generation = live.generation;
        let requests = s.set_enabled(false);
        assert_eq!(requests, [LiveRequest::Cancel { token: live }]);
        assert_eq!(s.pending_deadline(), None);
        assert!(s.note_edit(1_000).is_empty());
        // The cancelled run lands after the toggle: older generation →
        // superseded, nothing restarts while disabled.
        let (status, requests) = s.completed(live, 1_200, false);
        assert_eq!(status, LiveCompletion::Superseded(live));
        assert!(requests.is_empty());
        assert!(s.poll(10_000, false).is_empty());
        // A manual run survives a toggle-off.
        let mut s = enabled();
        let manual = match s.request_manual().as_slice() {
            [LiveRequest::StartManual { token }] => *token,
            other => panic!("expected StartManual, got {other:?}"),
        };
        assert!(s.set_enabled(false).is_empty());
        let (status, _) = s.completed(manual, 5_000, false);
        assert_eq!(status, LiveCompletion::Current(manual));
        let _ = generation;
    }

    #[test]
    fn invalidate_clears_queued_manual_and_pending() {
        let mut s = enabled();
        s.note_edit(0);
        let live = live_start(&s.poll(700, false));
        s.request_manual();
        // Invalidate while the live cancellation is in flight: the queued
        // manual belonged to the old context and must not start.
        let requests = s.invalidate();
        assert!(requests.is_empty()); // live already cancel-requested
        let (status, requests) = s.completed(live, 2_000, false);
        assert_eq!(status, LiveCompletion::Superseded(live));
        assert!(requests.is_empty());
        assert!(s.active().is_none());
    }

    #[test]
    fn delay_change_recomputes_from_edit_time() {
        let mut s = enabled();
        s.note_edit(1_000);
        assert_eq!(s.pending_deadline(), Some(1_700));
        s.set_delay(300);
        assert_eq!(s.pending_deadline(), Some(1_300));
        s.set_delay(5_000);
        assert_eq!(s.pending_deadline(), Some(6_000));
    }

    #[test]
    fn manual_completion_publishes_and_pending_live_starts() {
        let mut s = enabled();
        let manual = match s.request_manual().as_slice() {
            [LiveRequest::StartManual { token }] => *token,
            other => panic!("expected StartManual, got {other:?}"),
        };
        // Edits during the manual run stay pending.
        s.note_edit(100);
        let (status, requests) = s.completed(manual, 900, false);
        assert_eq!(status, LiveCompletion::Current(manual));
        // Deadline (800) already elapsed → live starts immediately.
        let live = live_start(&requests);
        let (status, _) = s.completed(live, 2_000, false);
        assert_eq!(status, LiveCompletion::Current(live));
    }
}
