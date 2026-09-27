//! The Regex workspace: test a Rust `regex` crate pattern against text, inspect
//! every match and capture, and preview a replacement with the actual dialect
//! and replacement syntax shown in the UI.
//!
//! This workspace never emulates ICU: unsupported look-around and
//! backreferences surface as diagnostics. Evaluation is bounded by the named
//! limits in the core contract, evaluated off the UI thread and revision-gated
//! through the shared [`Session`], so only the current revision publishes a
//! result or a History snapshot.

use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use gpui::prelude::*;
use gpui::{div, App, Context, Entity, IntoElement, Render, Subscription, Window};
use gpui_kit::component::Disableable as _;
use gpui_kit::component::{
    button::{Button, ButtonVariants as _},
    input::{Input, InputEvent, InputState, TextareaState},
    list::ListState,
    ActiveTheme as _,
};
use sofdevtool_core::session::{Session, SubmitOutcome};
use sofdevtool_core::utilities::regex::{
    evaluate_with_cancellation, CaptureInfo, Regex, RegexEvaluation, RegexFlags, RegexLimits,
    RegexRequest, RegexSnapshot, RUST_REGEX_DIALECT_NOTE, RUST_REGEX_ENGINE_LABEL,
    RUST_REGEX_FLAGS_NOTE, RUST_REGEX_REPLACEMENT_NOTE,
};
use sofdevtool_core::utility::Utility;

use crate::clipboard::Clipboard;
use crate::history::{HistoryEntry, HistoryRecorder, HistorySubscription, HistoryViewState};
use crate::registry::{UtilityId, WorkspaceViews};
use crate::ui;
use crate::workbench::Workbench;
use crate::workspace_layout::{HistoryPlacement, WorkspaceLayout};

const DEBOUNCE: Duration = Duration::from_millis(200);
const COMPLETION_POLL: Duration = Duration::from_millis(30);

type RegexSession = Session<Regex>;

struct WorkRequest {
    revision: u64,
    request: RegexRequest,
    ready_at: Instant,
}

#[derive(Default)]
struct WorkState {
    pending: Option<WorkRequest>,
    running_revision: Option<u64>,
    stopped: bool,
}

/// Exactly one engine thread and one replaceable pending request per workspace.
/// The engine may finish an individual call after cancellation, but no obsolete
/// result escapes the worker or the session's final revision check.
struct RegexWorker {
    state: Arc<(Mutex<WorkState>, Condvar)>,
    current_revision: Arc<AtomicU64>,
    completed: Arc<Mutex<Option<(u64, RegexEvaluation)>>>,
}

impl RegexWorker {
    fn new() -> Self {
        Self::with_evaluator(|request, cancelled| {
            evaluate_with_cancellation(request, &RegexLimits::default(), cancelled)
        })
    }

    fn with_evaluator(
        evaluator: impl Fn(&RegexRequest, &dyn Fn() -> bool) -> Option<RegexEvaluation> + Send + 'static,
    ) -> Self {
        let state = Arc::new((Mutex::new(WorkState::default()), Condvar::new()));
        let current_revision = Arc::new(AtomicU64::new(0));
        let completed = Arc::new(Mutex::new(None));
        let thread_state = Arc::clone(&state);
        let thread_revision = Arc::clone(&current_revision);
        let thread_completed = Arc::clone(&completed);
        std::thread::Builder::new()
            .name("sofdevtool-regex".to_owned())
            .spawn(move || {
                let (lock, wake) = &*thread_state;
                loop {
                    let mut work = lock.lock().expect("Regex worker state poisoned");
                    while work.pending.is_none() && !work.stopped {
                        work = wake.wait(work).expect("Regex worker state poisoned");
                    }
                    if work.stopped {
                        break;
                    }
                    let wait = work
                        .pending
                        .as_ref()
                        .expect("pending request")
                        .ready_at
                        .saturating_duration_since(Instant::now());
                    if !wait.is_zero() {
                        drop(
                            wake.wait_timeout(work, wait)
                                .expect("Regex worker state poisoned"),
                        );
                        continue;
                    }
                    let request = work.pending.take().expect("pending request");
                    work.running_revision = Some(request.revision);
                    drop(work);

                    let obsolete = || thread_revision.load(Ordering::Acquire) != request.revision;
                    let result = if obsolete() {
                        None
                    } else {
                        evaluator(&request.request, &obsolete)
                    };
                    if let Some(evaluation) = result {
                        if !obsolete() {
                            let mut slot = thread_completed.lock().expect("Regex result poisoned");
                            *slot = Some((request.revision, evaluation));
                        }
                    }
                    lock.lock()
                        .expect("Regex worker state poisoned")
                        .running_revision = None;
                }
            })
            .expect("Regex worker thread must start");
        Self {
            state,
            current_revision,
            completed,
        }
    }

    fn submit(&self, revision: u64, request: RegexRequest) {
        self.current_revision.store(revision, Ordering::Release);
        self.completed.lock().expect("Regex result poisoned").take();
        let (lock, wake) = &*self.state;
        let mut state = lock.lock().expect("Regex worker state poisoned");
        state.pending = Some(WorkRequest {
            revision,
            request,
            ready_at: Instant::now() + DEBOUNCE,
        });
        wake.notify_one();
    }

    fn invalidate(&self, revision: u64) {
        self.current_revision.store(revision, Ordering::Release);
        self.completed.lock().expect("Regex result poisoned").take();
        let (lock, wake) = &*self.state;
        let mut state = lock.lock().expect("Regex worker state poisoned");
        state.pending = None;
        wake.notify_one();
    }

    fn take_completed(&self) -> Option<(u64, RegexEvaluation)> {
        self.completed.lock().expect("Regex result poisoned").take()
    }

    fn has_current_work(&self) -> bool {
        let revision = self.current_revision.load(Ordering::Acquire);
        let (lock, _) = &*self.state;
        let active = {
            let state = lock.lock().expect("Regex worker state poisoned");
            state.pending.as_ref().map(|job| job.revision) == Some(revision)
                || state.running_revision == Some(revision)
        };
        // Completion is published before running_revision is cleared. Read
        // this slot second, so a completion between observations stays visible.
        active
            || self
                .completed
                .lock()
                .expect("Regex result poisoned")
                .is_some()
    }
}

impl Drop for RegexWorker {
    fn drop(&mut self) {
        self.current_revision.store(u64::MAX, Ordering::Release);
        let (lock, wake) = &*self.state;
        let mut state = lock.lock().expect("Regex worker state poisoned");
        state.pending = None;
        state.stopped = true;
        wake.notify_one();
    }
}

/// One boolean flag this workspace can set on the Rust regex engine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Flag {
    CaseInsensitive,
    MultiLine,
    DotMatchesNewLine,
    IgnoreWhitespace,
    SwapGreed,
    Crlf,
}

impl Flag {
    const ALL: [Flag; 6] = [
        Flag::CaseInsensitive,
        Flag::MultiLine,
        Flag::DotMatchesNewLine,
        Flag::IgnoreWhitespace,
        Flag::SwapGreed,
        Flag::Crlf,
    ];

    fn index(self) -> usize {
        match self {
            Flag::CaseInsensitive => 0,
            Flag::MultiLine => 1,
            Flag::DotMatchesNewLine => 2,
            Flag::IgnoreWhitespace => 3,
            Flag::SwapGreed => 4,
            Flag::Crlf => 5,
        }
    }

    /// A distinct, mnemonic label; stable Button identities use flag indices.
    fn label(self) -> &'static str {
        match self {
            Flag::CaseInsensitive => "i  case-insensitive",
            Flag::MultiLine => "m  multi-line",
            Flag::DotMatchesNewLine => "s  dot matches newline",
            Flag::IgnoreWhitespace => "x  ignore whitespace",
            Flag::SwapGreed => "U  swap greed",
            Flag::Crlf => "R  CRLF mode",
        }
    }

    fn enabled(self, flags: RegexFlags) -> bool {
        match self {
            Flag::CaseInsensitive => flags.case_insensitive,
            Flag::MultiLine => flags.multi_line,
            Flag::DotMatchesNewLine => flags.dot_matches_new_line,
            Flag::IgnoreWhitespace => flags.ignore_whitespace,
            Flag::SwapGreed => flags.swap_greed,
            Flag::Crlf => flags.crlf,
        }
    }

    fn toggled(self, mut flags: RegexFlags) -> RegexFlags {
        match self {
            Flag::CaseInsensitive => flags.case_insensitive = !flags.case_insensitive,
            Flag::MultiLine => flags.multi_line = !flags.multi_line,
            Flag::DotMatchesNewLine => flags.dot_matches_new_line = !flags.dot_matches_new_line,
            Flag::IgnoreWhitespace => flags.ignore_whitespace = !flags.ignore_whitespace,
            Flag::SwapGreed => flags.swap_greed = !flags.swap_greed,
            Flag::Crlf => flags.crlf = !flags.crlf,
        }
        flags
    }
}

/// Builds the Regex workspace as a type-erased view for the Workbench.
pub fn construct(
    window: &mut Window,
    cx: &mut Context<Workbench>,
    clipboard: Rc<dyn Clipboard>,
    history: Rc<HistoryRecorder>,
    layout: Entity<WorkspaceLayout>,
) -> WorkspaceViews {
    let workspace =
        cx.new(|cx| RegexWorkspace::new_with_layout(window, cx, clipboard, history, layout));
    let inspector = cx.new(|cx| {
        ui::HistoryInspector::new(
            workspace.clone(),
            |workspace, cx| workspace.render_history(cx).into_any_element(),
            cx,
        )
    });
    WorkspaceViews {
        body: workspace.into(),
        history: inspector.into(),
    }
}

/// The Regex workspace: pattern/text/replacement editors are GPUI Kit inputs;
/// History selection is the app-owned kit `List` composition.
pub struct RegexWorkspace {
    pattern: Entity<InputState>,
    text: Entity<TextareaState>,
    replacement: Entity<InputState>,
    preview: Entity<TextareaState>,
    clipboard: Rc<dyn Clipboard>,
    history: Rc<HistoryRecorder>,
    flags: RegexFlags,
    session: RegexSession,
    worker: RegexWorker,
    poller_started: bool,
    display_epoch: u64,
    copied: bool,
    suppress_changes: bool,
    history_view: HistoryViewState,
    layout: Entity<WorkspaceLayout>,
    _layout_subscription: Subscription,
    history_list: Entity<ListState<ui::HistoryListDelegate>>,
    _history_subscription: HistorySubscription,
    _subscriptions: Vec<Subscription>,
}

impl RegexWorkspace {
    #[cfg(test)]
    fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        clipboard: Rc<dyn Clipboard>,
        history: Rc<HistoryRecorder>,
    ) -> Self {
        let layout = cx.new(|_| WorkspaceLayout::load(None));
        Self::new_with_layout(window, cx, clipboard, history, layout)
    }

    fn new_with_layout(
        window: &mut Window,
        cx: &mut Context<Self>,
        clipboard: Rc<dyn Clipboard>,
        history: Rc<HistoryRecorder>,
        layout: Entity<WorkspaceLayout>,
    ) -> Self {
        let layout_subscription = cx.observe(&layout, |_, _, cx| cx.notify());
        let pattern = cx.new(|cx| InputState::new(window, cx));
        let text = cx.new(|cx| TextareaState::new(window, cx));
        let replacement = cx.new(|cx| InputState::new(window, cx));
        let preview = cx.new(|cx| TextareaState::new(window, cx));
        let subscriptions = vec![
            cx.subscribe_in(
                &pattern,
                window,
                |this, _entity, event: &InputEvent, window, cx| {
                    if matches!(event, InputEvent::Change) {
                        this.schedule(window, cx);
                    }
                },
            ),
            cx.subscribe_in(
                &text,
                window,
                |this, _entity, event: &InputEvent, window, cx| {
                    if matches!(event, InputEvent::Change) {
                        this.schedule(window, cx);
                    }
                },
            ),
            cx.subscribe_in(
                &replacement,
                window,
                |this, _entity, event: &InputEvent, window, cx| {
                    if matches!(event, InputEvent::Change) {
                        this.schedule(window, cx);
                    }
                },
            ),
        ];
        let history_view = HistoryViewState::load(&history, Regex::ID);
        let weak = cx.weak_entity();
        let history_subscription = history.subscribe(Regex::ID, move |cx| {
            weak.update(cx, |this, cx| this.reconcile_history(cx)).ok();
        });
        let weak = cx.weak_entity();
        let history_list = ui::history_state(
            window,
            cx,
            "No retained operations yet.",
            Rc::new(move |id, _window, cx| {
                weak.update(cx, |this, cx| {
                    if this.history_view.select(id) {
                        cx.notify();
                    }
                })
                .ok();
            }),
        );
        let this = Self {
            pattern,
            text,
            replacement,
            preview,
            clipboard,
            history,
            flags: RegexFlags::default(),
            session: RegexSession::new(),
            worker: RegexWorker::new(),
            poller_started: false,
            display_epoch: u64::MAX,
            copied: false,
            suppress_changes: false,
            history_view,
            layout,
            _layout_subscription: layout_subscription,
            history_list,
            _history_subscription: history_subscription,
            _subscriptions: subscriptions,
        };
        this.sync_history(cx);
        this
    }

    /// Reflects the owning view's History rows and selection into the kit list.
    fn sync_history(&self, cx: &mut Context<Self>) {
        ui::history_set_rows(&self.history_list, self.history_items(), cx);
        ui::history_set_selected(&self.history_list, self.history_view.selected.clone(), cx);
    }

    fn request(&self, cx: &App) -> RegexRequest {
        RegexRequest {
            pattern: self.pattern.read(cx).value().to_string(),
            text: self.text.read(cx).value().to_string(),
            flags: self.flags,
            replacement: self.replacement.read(cx).value().to_string(),
        }
    }

    /// Submits the current request. A changed request clears the visible result
    /// immediately and replaces the worker's single pending request.
    fn schedule(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.suppress_changes {
            return;
        }
        let SubmitOutcome::Scheduled(revision) = self.session.submit(self.request(cx)) else {
            return;
        };
        self.copied = false;
        self.worker.submit(
            revision,
            self.session.request().expect("submitted request").clone(),
        );
        self.sync_display(window, cx);
        cx.notify();
        if !self.poller_started {
            self.poller_started = true;
            let executor = cx.background_executor().clone();
            cx.spawn(async move |this, cx| loop {
                executor.timer(COMPLETION_POLL).await;
                let keep_polling = this.update(cx, |this, cx| {
                    if let Some((revision, evaluation)) = this.worker.take_completed() {
                        if this.session.publish(revision, evaluation).is_some() {
                            this.record_settled(cx);
                            cx.notify();
                        }
                    }
                    let active = this.worker.has_current_work();
                    if !active {
                        this.poller_started = false;
                    }
                    active
                });
                if !matches!(keep_polling, Ok(true)) {
                    break;
                }
            })
            .detach();
        }
    }

    /// Records the one settled valid operation for the current revision, if any.
    fn record_settled(&mut self, cx: &mut Context<Self>) {
        let Some(snapshot) = self.session.take_snapshot() else {
            return;
        };
        let payload = serde_json::to_value(&snapshot).expect("a Regex snapshot serializes");
        let result = self
            .history
            .record(Regex::ID, Regex::SNAPSHOT_VERSION, payload);
        self.history_view
            .apply_record(&self.history, Regex::ID, result);
        self.sync_history(cx);
        self.history.notify_status(cx);
        cx.notify();
    }

    fn reconcile_history(&mut self, cx: &mut Context<Self>) {
        self.history_view.reconcile(&self.history, Regex::ID);
        self.sync_history(cx);
        cx.notify();
    }

    /// Mirrors the settled replacement preview into the read-only editor, but
    /// only when the evaluation epoch changed. Obsolete results are never shown.
    fn sync_display(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let epoch = self.session.evaluation_epoch();
        if self.display_epoch == epoch {
            return;
        }
        self.display_epoch = epoch;
        let replacement = match self.session.evaluation() {
            RegexEvaluation::Valid { replacement, .. } => replacement.clone().unwrap_or_default(),
            _ => String::new(),
        };
        self.preview
            .update(cx, |state, cx| state.set_value(replacement, window, cx));
    }

    fn toggle_flag(&mut self, flag: Flag, window: &mut Window, cx: &mut Context<Self>) {
        self.flags = flag.toggled(self.flags);
        self.schedule(window, cx);
    }

    fn paste(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = self.clipboard.read_text(cx) {
            self.copied = false;
            self.text
                .update(cx, |state, cx| state.replace_all(text, window, cx));
        }
    }

    fn copy_result(&mut self, cx: &mut Context<Self>) {
        if let Some(replacement) = self.session.evaluation().replacement() {
            let replacement = replacement.to_owned();
            self.clipboard.write_text(&replacement, cx);
            self.copied = true;
            cx.notify();
        }
    }

    fn clear(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.copied = false;
        self.suppress_changes = true;
        self.pattern
            .update(cx, |state, cx| state.set_value("", window, cx));
        self.text
            .update(cx, |state, cx| state.set_value("", window, cx));
        self.replacement
            .update(cx, |state, cx| state.set_value("", window, cx));
        self.suppress_changes = false;
        self.session.clear();
        self.worker.invalidate(self.session.revision());
        self.display_epoch = u64::MAX;
        self.copied = false;
        self.sync_display(window, cx);
        cx.notify();
    }

    fn request_restore(
        &mut self,
        entry: HistoryEntry,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(snapshot) = decode_snapshot(&entry) else {
            self.history_view.error =
                Some("This History entry uses a snapshot version this build cannot read.".into());
            cx.notify();
            return;
        };
        let current = self.request(cx);
        let non_empty = !(current.pattern.is_empty()
            && current.text.is_empty()
            && current.replacement.is_empty());
        if non_empty && current != snapshot.request {
            self.history_view.pending_restore = Some(entry);
            let weak = cx.weak_entity();
            ui::confirm_dialog(
                window,
                cx,
                "Restore History entry",
                "Restoring this History entry replaces the current non-empty Regex session.",
                "Restore",
                "Cancel",
                {
                    let weak = weak.clone();
                    move |window, cx| {
                        let _ = weak.update(cx, |this, cx| this.confirm_restore(window, cx));
                    }
                },
                {
                    let weak = weak.clone();
                    move |_window, cx| {
                        let _ = weak.update(cx, |this, cx| this.cancel_restore(cx));
                    }
                },
            );
            cx.notify();
        } else {
            self.apply_restore(snapshot, window, cx);
        }
    }

    fn apply_restore(
        &mut self,
        snapshot: RegexSnapshot,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.suppress_changes = true;
        self.pattern.update(cx, |state, cx| {
            state.set_value(snapshot.request.pattern.clone(), window, cx)
        });
        self.text.update(cx, |state, cx| {
            state.set_value(snapshot.request.text.clone(), window, cx)
        });
        self.replacement.update(cx, |state, cx| {
            state.set_value(snapshot.request.replacement.clone(), window, cx)
        });
        self.flags = snapshot.request.flags;
        self.session.restore(snapshot);
        self.worker.invalidate(self.session.revision());
        self.display_epoch = u64::MAX;
        self.suppress_changes = false;
        self.history_view.pending_restore = None;
        self.copied = false;
        self.sync_history(cx);
        self.sync_display(window, cx);
        cx.notify();
    }

    fn confirm_restore(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(entry) = self.history_view.pending_restore.clone() {
            if self.history_view.retained(&self.history, Regex::ID, &entry) {
                if let Some(snapshot) = decode_snapshot(&entry) {
                    self.apply_restore(snapshot, window, cx);
                }
            } else {
                self.reconcile_history(cx);
            }
        }
    }

    fn cancel_restore(&mut self, cx: &mut Context<Self>) {
        self.history_view.pending_restore = None;
        cx.notify();
    }

    fn restore_selected(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(selected) = self.history_view.selected.clone() else {
            return;
        };
        if let Some(entry) = self
            .history_view
            .entries
            .iter()
            .find(|entry| entry.id == selected)
            .cloned()
        {
            if self.history_view.retained(&self.history, Regex::ID, &entry) {
                self.request_restore(entry, window, cx);
            } else {
                self.reconcile_history(cx);
            }
        }
    }

    fn history_items(&self) -> Vec<ui::HistoryRow> {
        self.history_view
            .entries
            .iter()
            .map(|entry| {
                let snapshot = decode_snapshot(entry);
                ui::HistoryRow {
                    id: entry.id.clone(),
                    label: entry.captured_at.clone(),
                    preview: snapshot
                        .as_ref()
                        .map(|snapshot| {
                            format!(
                                "{} match(es) · {}",
                                snapshot.matches.len(),
                                preview_line(&snapshot.replacement)
                            )
                        })
                        .unwrap_or_else(|| "Unavailable snapshot".to_owned()),
                    status: snapshot.is_none().then(|| "Unavailable".to_owned()),
                    selectable: true,
                }
            })
            .collect()
    }

    fn flag_button(&self, flag: Flag, cx: &mut Context<Self>) -> Button {
        Button::new(format!("regex.flag.{}", flag.index()))
            .label(flag.label())
            .when(flag.enabled(self.flags), |button| button.primary())
            .on_click(cx.listener(move |this, _event, window, cx| {
                this.toggle_flag(flag, window, cx);
            }))
    }

    fn render_matches(&self, cx: &App) -> impl IntoElement {
        let theme = cx.theme().clone();
        let mut list = div()
            .id("regex.matches-list")
            .flex()
            .flex_col()
            .gap_2()
            .flex_1()
            .min_h_0()
            .w_full()
            .overflow_y_scroll();
        for matched in self.session.evaluation().matches() {
            let mut row = div()
                .flex()
                .flex_col()
                .gap_1()
                .w_full()
                .px_2()
                .py_1()
                .rounded_md()
                .border_1()
                .border_color(theme.border)
                .bg(theme.secondary)
                .child(div().text_sm().text_color(theme.foreground).child(format!(
                    "{}. {}",
                    matched.ordinal + 1,
                    escaped(&matched.value)
                )))
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(format!(
                            "bytes {}..{}",
                            matched.range.start, matched.range.end
                        )),
                );
            for capture in &matched.captures {
                row = row.child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(capture_line(capture)),
                );
            }
            list = list.child(row);
        }
        list
    }

    fn render_diagnostics(&self, cx: &App) -> impl IntoElement {
        let mut column = div().flex().flex_col().gap_2().w_full();
        for diagnostic in self.session.evaluation().diagnostics() {
            let severity = match diagnostic.severity {
                sofdevtool_core::diagnostic::Severity::Error => ui::DiagnosticSeverity::Error,
                sofdevtool_core::diagnostic::Severity::Warning => ui::DiagnosticSeverity::Warning,
            };
            let location = diagnostic.location.map(|l| (l.line, l.column));
            column = column.child(ui::diagnostic_banner(
                cx,
                severity,
                &diagnostic.message,
                location,
            ));
        }
        column
    }

    fn render_history(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let selected = self.history_view.selected.clone();
        let restore_enabled = selected
            .as_ref()
            .and_then(|id| {
                self.history_view
                    .entries
                    .iter()
                    .find(|entry| &entry.id == id)
            })
            .is_some_and(|entry| decode_snapshot(entry).is_some());
        let restore = Button::new("regex.history.restore-selected")
            .label("Restore selected")
            .disabled(!restore_enabled)
            .on_click(cx.listener(|this, _event, window, cx| {
                this.restore_selected(window, cx);
            }));
        div()
            .flex()
            .flex_col()
            .w_64()
            .min_h_0()
            .p_3()
            .gap_2()
            .border_l_1()
            .border_color(theme.border)
            .bg(theme.popover)
            .child(ui::history_panel(
                cx,
                &self.history_list,
                "History",
                format!("{}/25", self.history_view.entries.len()),
                restore,
            ))
    }
}

impl Render for RegexWorkspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_display(window, cx);
        let theme = cx.theme().clone();
        let can_copy = self.session.evaluation().is_valid_operation();
        let neutral = matches!(self.session.evaluation(), RegexEvaluation::Empty);
        let pending = neutral
            && self
                .session
                .request()
                .map(|request| !(request.pattern.is_empty() && request.text.is_empty()))
                .unwrap_or(false);

        let mut flags_row = div().flex().flex_row().items_center().flex_wrap().gap_2();
        for flag in Flag::ALL {
            flags_row = flags_row.child(self.flag_button(flag, cx));
        }

        let pattern_row = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .child(
                div()
                    .w_24()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child("Pattern"),
            )
            .child(
                div().flex_1().min_w_0().child(
                    Input::new(&self.pattern)
                        .accessibility_id("regex.pattern")
                        .w_full(),
                ),
            );

        let replacement_row = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .child(
                div()
                    .w_24()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child("Replacement template"),
            )
            .child(
                div().flex_1().min_w_0().child(
                    Input::new(&self.replacement)
                        .accessibility_id("regex.replacement")
                        .w_full(),
                ),
            );

        let guidance = div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .text_xs()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(theme.primary)
                    .child(RUST_REGEX_ENGINE_LABEL),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(RUST_REGEX_DIALECT_NOTE),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(RUST_REGEX_FLAGS_NOTE),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(RUST_REGEX_REPLACEMENT_NOTE),
            );

        let toolbar = div()
            .flex()
            .flex_row()
            .items_center()
            .flex_wrap()
            .gap_2()
            .child(div().flex_1())
            .child(ui::copy_feedback(cx, self.copied, "Copied to Clipboard"))
            .child(
                Button::new("regex.paste")
                    .label("Paste")
                    .on_click(cx.listener(|this, _event, window, cx| {
                        this.paste(window, cx);
                    })),
            )
            .child(
                Button::new("regex.copy-result")
                    .label("Copy Result")
                    .disabled(!can_copy)
                    .on_click(cx.listener(|this, _event, _window, cx| {
                        this.copy_result(cx);
                    })),
            )
            .child(
                Button::new("regex.clear")
                    .label("Clear")
                    .on_click(cx.listener(|this, _event, window, cx| {
                        this.clear(window, cx);
                    })),
            );

        let mut column = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .bg(theme.background)
            .text_color(theme.foreground)
            .gap_3()
            .child(guidance)
            .child(flags_row)
            .child(pattern_row)
            .child(replacement_row)
            .child(toolbar);

        if let Some(error) = self.history_view.error.clone() {
            column = column.child(ui::diagnostic_banner(
                cx,
                ui::DiagnosticSeverity::Warning,
                &format!("Regex History: {error}"),
                None,
            ));
        }

        let preview_body = if pending {
            ui::empty_state(cx, "Evaluating…").into_any_element()
        } else if neutral {
            ui::empty_state(cx, "Enter a pattern and test text to begin").into_any_element()
        } else {
            ui::multiline_editor(&self.preview, true, "regex.replacement-preview")
                .into_any_element()
        };

        let matches_body = if pending {
            ui::empty_state(cx, "Evaluating…").into_any_element()
        } else {
            match self.session.evaluation() {
                RegexEvaluation::Empty => {
                    ui::empty_state(cx, "Enter a pattern and test text to begin").into_any_element()
                }
                RegexEvaluation::Valid { matches, .. } if matches.is_empty() => {
                    ui::empty_state(cx, "No matches").into_any_element()
                }
                RegexEvaluation::Valid { .. } => self.render_matches(cx).into_any_element(),
                RegexEvaluation::Invalid { .. } => {
                    ui::empty_state(cx, "No result was published for the current input")
                        .into_any_element()
                }
            }
        };

        let left = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .gap_3()
            .child(ui::panel(
                cx,
                "Test Text",
                "exact UTF-8 bytes",
                ui::multiline_editor(&self.text, false, "regex.text"),
            ))
            .child(ui::panel(
                cx,
                "Replacement Preview",
                "read-only, copyable",
                preview_body,
            ));

        let mut workspace = div()
            .flex()
            .flex_row()
            .gap_3()
            .flex_1()
            .min_h_0()
            .child(left)
            .child(ui::panel(
                cx,
                "Matches & Captures",
                RUST_REGEX_ENGINE_LABEL,
                matches_body,
            ));
        if self.layout.read(cx).placement(UtilityId::Regex) == HistoryPlacement::Inline {
            workspace = workspace.child(self.render_history(cx));
        }
        column.child(workspace).child(self.render_diagnostics(cx))
    }
}

fn decode_snapshot(entry: &HistoryEntry) -> Option<RegexSnapshot> {
    if entry.snapshot_version != Regex::SNAPSHOT_VERSION {
        return None;
    }
    serde_json::from_value(entry.payload.clone()).ok()
}

/// One display line for a capture: `$name (\$index) = value [start..end)`, or
/// `(unmatched)` when the group did not participate.
fn capture_line(capture: &CaptureInfo) -> String {
    let name = match &capture.name {
        Some(name) => format!("{name} (${})", capture.index),
        None => format!("${}", capture.index),
    };
    match (&capture.value, capture.range) {
        (Some(value), Some(range)) => {
            format!(
                "  {name} = {} [{}..{})",
                escaped(value),
                range.start,
                range.end
            )
        }
        _ => format!("  {name} = (unmatched)"),
    }
}

/// Renders an exact value on one line; control characters stay visible.
fn escaped(text: &str) -> String {
    if text.is_empty() {
        return "(empty)".to_owned();
    }
    text.replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn preview_line(output: &str) -> String {
    let single_line = output.replace('\n', " ");
    let trimmed = single_line.trim();
    let mut preview: String = trimmed.chars().take(80).collect();
    if trimmed.chars().count() > 80 {
        preview.push('…');
    }
    if preview.is_empty() {
        "Empty result".to_owned()
    } else {
        preview
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    use gpui::VisualTestContext;

    use sofdevtool_core::utilities::regex::evaluate;

    use crate::history::{HistoryStore, SystemClock};

    struct TestClipboard;

    impl Clipboard for TestClipboard {
        fn read_text(&self, _cx: &mut App) -> Option<String> {
            None
        }

        fn write_text(&self, _text: &str, _cx: &mut App) {}
    }

    fn test_root() -> PathBuf {
        // A process-wide counter guarantees distinct stores even when two tests
        // call this within the same clock tick, which the parallel test harness
        // otherwise allows and which would let `clear_utility` delete a sibling's
        // records.
        static NEXT_ROOT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        std::env::temp_dir().join(format!(
            "sofdevtool-regex-history-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT_ROOT.fetch_add(1, Ordering::Relaxed),
        ))
    }

    fn await_completion(worker: &RegexWorker) -> (u64, RegexEvaluation) {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(completion) = worker.take_completed() {
                return completion;
            }
            assert!(Instant::now() < deadline, "Regex worker did not finish");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn unread_completion_keeps_the_poller_active_after_engine_work_ends() {
        let worker = RegexWorker::new();
        worker.submit(1, RegexRequest::new("word", "word"));
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            let (lock, _) = &*worker.state;
            let running = lock.lock().expect("worker state").running_revision;
            let completed = worker.completed.lock().expect("result slot").is_some();
            if running.is_none() && completed {
                break;
            }
            assert!(Instant::now() < deadline, "worker did not complete");
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(worker.has_current_work(), "UI must still read completion");
        assert_eq!(worker.take_completed().expect("completion").0, 1);
        assert!(!worker.has_current_work(), "idle poller may now stop");
    }

    #[test]
    fn worker_keeps_one_running_and_one_replaceable_pending_request() {
        use std::sync::mpsc;

        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let calls = Arc::new(Mutex::new(Vec::new()));
        let calls_in_worker = Arc::clone(&calls);
        let worker = RegexWorker::with_evaluator(move |request, cancelled| {
            calls_in_worker
                .lock()
                .expect("call log")
                .push(request.pattern.clone());
            if request.pattern == "slow" {
                started_tx.send(()).expect("start observer");
                release_rx.recv().expect("release slow evaluation");
            }
            if cancelled() {
                None
            } else {
                Some(evaluate(request))
            }
        });
        let mut session = RegexSession::new();
        let SubmitOutcome::Scheduled(slow) = session.submit(RegexRequest::new("slow", "slow"))
        else {
            panic!("slow revision");
        };
        worker.submit(slow, session.request().expect("request").clone());
        started_rx
            .recv_timeout(Duration::from_secs(3))
            .expect("slow work starts on engine thread");
        assert!(worker.has_current_work());

        for index in 0..40 {
            let request = RegexRequest::new(format!("intermediate-{index}"), "text");
            let SubmitOutcome::Scheduled(revision) = session.submit(request.clone()) else {
                panic!("new intermediate revision");
            };
            worker.submit(revision, request);
        }
        let winner = RegexRequest::new("winner", "winner");
        let SubmitOutcome::Scheduled(winning_revision) = session.submit(winner.clone()) else {
            panic!("winning revision");
        };
        worker.submit(winning_revision, winner);
        assert_eq!(session.evaluation(), &RegexEvaluation::Empty);
        assert!(session.take_snapshot().is_none());
        release_tx.send(()).expect("release slow work");

        let (revision, evaluation) = await_completion(&worker);
        assert_eq!(revision, winning_revision);
        assert_eq!(evaluation.matches().len(), 1);
        assert!(session
            .publish(slow, evaluate(&RegexRequest::new("slow", "slow")))
            .is_none());
        assert!(session.publish(revision, evaluation).is_some());
        assert!(session.take_snapshot().is_some());
        assert!(session.take_snapshot().is_none());
        assert_eq!(
            *calls.lock().expect("call log"),
            vec!["slow".to_owned(), "winner".to_owned()]
        );
    }

    #[test]
    fn clear_and_restore_invalidate_running_or_pending_work() {
        use std::sync::mpsc;

        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let worker = RegexWorker::with_evaluator(move |request, cancelled| {
            started_tx.send(()).expect("start observer");
            release_rx.recv().expect("release evaluation");
            if cancelled() {
                None
            } else {
                Some(evaluate(request))
            }
        });
        let mut session = RegexSession::new();
        let request = RegexRequest::new("first", "first");
        let SubmitOutcome::Scheduled(first) = session.submit(request.clone()) else {
            panic!("first revision");
        };
        worker.submit(first, request);
        started_rx
            .recv_timeout(Duration::from_secs(3))
            .expect("running work starts");

        session.clear();
        worker.invalidate(session.revision());
        assert_eq!(session.evaluation(), &RegexEvaluation::Empty);
        assert!(
            !worker.has_current_work(),
            "obsolete running work needs no UI poller"
        );
        let pending = RegexRequest::new("pending", "pending");
        let SubmitOutcome::Scheduled(pending_revision) = session.submit(pending.clone()) else {
            panic!("pending revision");
        };
        worker.submit(pending_revision, pending);

        let restored_request = RegexRequest::new("restored", "restored");
        let restored_evaluation = evaluate(&restored_request);
        let snapshot = <Regex as Utility>::snapshot(&restored_request, &restored_evaluation)
            .expect("valid snapshot");
        session.restore(snapshot);
        worker.invalidate(session.revision());
        assert!(
            !worker.has_current_work(),
            "restore has no current engine work"
        );
        release_tx.send(()).expect("release running work");

        assert!(session
            .publish(first, evaluate(&RegexRequest::new("first", "first")))
            .is_none());
        assert!(session
            .publish(
                pending_revision,
                evaluate(&RegexRequest::new("pending", "pending"))
            )
            .is_none());
        assert_eq!(session.evaluation().matches()[0].value, "restored");
        assert!(session.take_snapshot().is_none());
        // A fresh request proves the worker drained the old call and did not
        // retain the cancelled pending request.
        let fresh = RegexRequest::new("fresh", "fresh");
        let SubmitOutcome::Scheduled(fresh_revision) = session.submit(fresh.clone()) else {
            panic!("fresh revision");
        };
        worker.submit(fresh_revision, fresh);
        started_rx
            .recv_timeout(Duration::from_secs(3))
            .expect("fresh work starts");
        release_tx.send(()).expect("release fresh work");
        let (revision, evaluation) = await_completion(&worker);
        assert_eq!(revision, fresh_revision);
        assert!(session.publish(revision, evaluation).is_some());
    }

    #[test]
    fn history_snapshot_decoding_rejects_unknown_versions() {
        let request = RegexRequest::new(r"\w+", "one two");
        let evaluation = evaluate(&request);
        let snapshot =
            <Regex as Utility>::snapshot(&request, &evaluation).expect("valid operation snapshots");
        let entry = HistoryEntry {
            id: "1".to_owned(),
            captured_at: "2026-01-01T00:00:00Z".to_owned(),
            utility_id: Regex::ID.to_owned(),
            snapshot_version: Regex::SNAPSHOT_VERSION,
            payload: serde_json::to_value(&snapshot).expect("snapshot serializes"),
        };
        assert!(decode_snapshot(&entry).is_some());
        let unknown = HistoryEntry {
            snapshot_version: 99,
            ..entry
        };
        assert!(decode_snapshot(&unknown).is_none());
    }

    #[test]
    fn invalid_operations_do_not_produce_a_replaceable_snapshot() {
        let request = RegexRequest::new(r"a(?=b)", "ab");
        let evaluation = evaluate(&request);
        assert!(!evaluation.is_valid_operation());
        assert!(<Regex as Utility>::snapshot(&request, &evaluation).is_none());
    }

    #[test]
    fn capture_lines_distinguish_absent_empty_and_matched_groups() {
        let absent = CaptureInfo {
            index: 2,
            name: None,
            value: None,
            range: None,
        };
        assert!(capture_line(&absent).contains("(unmatched)"));

        let empty = CaptureInfo {
            index: 1,
            name: Some("word".to_owned()),
            value: Some(String::new()),
            range: Some(sofdevtool_core::utilities::regex::TextRange { start: 3, end: 3 }),
        };
        assert!(capture_line(&empty).contains("word ($1) = (empty)"));

        let matched = CaptureInfo {
            index: 0,
            name: None,
            value: Some("a\nb".to_owned()),
            range: Some(sofdevtool_core::utilities::regex::TextRange { start: 0, end: 3 }),
        };
        assert_eq!(capture_line(&matched), "  $0 = a\\nb [0..3)");
    }

    #[gpui::test]
    fn flag_and_history_keyboard_controls_restore_without_reexecution(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        let root = test_root();
        let history = Rc::new(HistoryRecorder::new(
            HistoryStore::new(root.clone()),
            Box::new(SystemClock::new()),
        ));
        let request = RegexRequest::new("word", "word");
        let snapshot = <Regex as Utility>::snapshot(&request, &evaluate(&request)).unwrap();
        let entry = HistoryEntry {
            id: "retained-regex".into(),
            captured_at: "2026-01-01T00:00:00Z".into(),
            utility_id: Regex::ID.into(),
            snapshot_version: Regex::SNAPSHOT_VERSION,
            payload: serde_json::to_value(snapshot).unwrap(),
        };
        history.store().record(entry.clone()).unwrap();
        let clipboard: Rc<dyn Clipboard> = Rc::new(TestClipboard);
        let mut captured = None;
        let window = cx.add_window(|window, cx| {
            let view = cx.new(|cx| RegexWorkspace::new(window, cx, clipboard, history.clone()));
            captured = Some(view.clone());
            gpui_kit::component::Root::new(view, window, cx)
        });
        let workspace = captured.unwrap();
        let mut cx = VisualTestContext::from_window(window.into(), cx);

        // The kit Button owns Enter/Space activation; drive the same domain
        // toggle it invokes so the flag state is asserted after a keyboard-
        // equivalent activation rather than through a stale focus handle.
        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                view.toggle_flag(Flag::CaseInsensitive, window, cx);
            });
        });
        assert!(workspace.read_with(&cx, |view, _| view.flags.case_insensitive));

        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                view.pattern
                    .update(cx, |state, cx| state.set_value("different", window, cx));
                view.text
                    .update(cx, |state, cx| state.set_value("different", window, cx));
                assert!(view.history_view.select(&entry.id));
                view.sync_history(cx);
            });
        });
        assert_eq!(
            workspace.read_with(&cx, |view, _| view.history_view.selected.clone()),
            Some(entry.id.clone())
        );
        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                view.restore_selected(window, cx);
            });
        });
        assert!(workspace.read_with(&cx, |view, _| view.history_view.pending_restore.is_some()));
        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                view.confirm_restore(window, cx);
            });
        });
        assert_eq!(
            workspace.read_with(&cx, |view, cx| view.pattern.read(cx).value().to_string()),
            "word"
        );
        assert_eq!(
            workspace.read_with(&cx, |view, cx| view.text.read(cx).value().to_string()),
            "word"
        );
        assert!(!workspace.read_with(&cx, |view, _| view.flags.case_insensitive));
        assert_eq!(
            workspace.read_with(&cx, |view, _| view.session.evaluation().matches().len()),
            1
        );
        assert!(!workspace.read_with(&cx, |view, _| view.worker.has_current_work()));
        assert_eq!(history.load(Regex::ID).unwrap().len(), 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[gpui::test]
    fn workspace_clear_preserves_inflight_revision_and_restore_never_rerecords(
        cx: &mut gpui::TestAppContext,
    ) {
        use std::sync::mpsc;

        cx.update(gpui_kit::init);
        let root = test_root();
        let history = Rc::new(HistoryRecorder::new(
            HistoryStore::new(root.clone()),
            Box::new(SystemClock::new()),
        ));
        let old_request = RegexRequest::new("old", "old");
        let old_snapshot = <Regex as Utility>::snapshot(&old_request, &evaluate(&old_request))
            .expect("old operation snapshots");
        let old_entry = HistoryEntry {
            id: "old-entry".into(),
            captured_at: "2026-01-01T00:00:00Z".into(),
            utility_id: Regex::ID.into(),
            snapshot_version: Regex::SNAPSHOT_VERSION,
            payload: serde_json::to_value(old_snapshot).unwrap(),
        };
        history.store().record(old_entry.clone()).unwrap();
        let clipboard: Rc<dyn Clipboard> = Rc::new(TestClipboard);
        let mut captured = None;
        let window = cx.add_window(|window, cx| {
            let view = cx.new(|cx| RegexWorkspace::new(window, cx, clipboard, history.clone()));
            captured = Some(view.clone());
            gpui_kit::component::Root::new(view, window, cx)
        });
        let workspace = captured.unwrap();
        let mut cx = VisualTestContext::from_window(window.into(), cx);
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                view.worker = RegexWorker::with_evaluator(move |request, cancelled| {
                    if request.pattern == "slow" {
                        started_tx.send(()).unwrap();
                        release_rx.recv().unwrap();
                    }
                    (!cancelled()).then(|| evaluate(request))
                });
                view.pattern
                    .update(cx, |state, cx| state.set_value("slow", window, cx));
                view.text
                    .update(cx, |state, cx| state.set_value("slow", window, cx));
                view.schedule(window, cx);
                assert!(view.history_view.select(&old_entry.id));
                view.sync_history(cx);
                view.history_view.pending_restore = Some(old_entry.clone());
            });
        });
        started_rx
            .recv_timeout(Duration::from_secs(3))
            .expect("slow evaluation starts");
        let slow_revision = workspace.read_with(&cx, |view, _| view.session.revision());
        cx.update(|window, cx| {
            history.clear_utility(Regex::ID, cx).unwrap();
            let view = workspace.read(cx);
            assert_eq!(view.session.revision(), slow_revision);
            assert!(view.worker.has_current_work());
            assert!(view.history_view.entries.is_empty());
            assert!(view.history_view.selected.is_none());
            assert!(view.history_view.pending_restore.is_none());
            workspace.update(cx, |view, cx| {
                view.history_view.pending_restore = Some(old_entry.clone());
                view.confirm_restore(window, cx);
                assert_eq!(view.pattern.read(cx).value().to_string(), "slow");
                view.pattern
                    .update(cx, |state, cx| state.set_value("winner", window, cx));
                view.text
                    .update(cx, |state, cx| state.set_value("winner", window, cx));
                view.schedule(window, cx);
                assert!(view.session.revision() > slow_revision);
            });
        });
        release_tx.send(()).unwrap();
        let (revision, evaluation) =
            workspace.read_with(&cx, |view, _| await_completion(&view.worker));
        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                assert_eq!(revision, view.session.revision());
                assert!(view.session.publish(revision, evaluation).is_some());
                view.sync_display(window, cx);
                view.record_settled(cx);
                view.record_settled(cx);
                assert_eq!(view.session.evaluation().matches()[0].value, "winner");
            });
        });
        let retained = history.load(Regex::ID).unwrap();
        assert_eq!(retained.len(), 1);
        let winner_entry = retained[0].clone();
        assert_eq!(
            decode_snapshot(&winner_entry).unwrap().request.pattern,
            "winner"
        );
        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                view.pattern
                    .update(cx, |state, cx| state.set_value("different", window, cx));
                view.text
                    .update(cx, |state, cx| state.set_value("different", window, cx));
                view.request_restore(winner_entry.clone(), window, cx);
                assert_eq!(
                    view.history_view.pending_restore,
                    Some(winner_entry.clone())
                );
                view.confirm_restore(window, cx);
                assert_eq!(view.pattern.read(cx).value().to_string(), "winner");
                assert_eq!(view.session.evaluation().matches()[0].value, "winner");
                assert!(view.session.take_snapshot().is_none());
            });
        });
        assert_eq!(history.load(Regex::ID).unwrap().len(), 1);
        fs::remove_dir_all(root).unwrap();
    }
}
