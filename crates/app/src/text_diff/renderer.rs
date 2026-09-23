use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gpui::{
    relative, App, Bounds, Element, ElementId, GlobalElementId, InspectorElementId, IntoElement,
    LayoutId, Pixels, Style, Window,
};
use wry::{
    dpi::{LogicalPosition, LogicalSize},
    Rect, WebView, WebViewBuilder,
};

// The canonical notice inventory is part of the local document handed to Wry,
// which makes it an actual packaged resource rather than an unused compiler
// input or a dependency on the retained Swift source tree.
const THIRD_PARTY_NOTICES: &str = include_str!("assets/THIRD_PARTY_NOTICES.md");

#[derive(Clone)]
pub(super) struct TextDiffRenderer(Rc<RefCell<RendererState>>);

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum RendererStatus {
    Loading,
    Ready,
    Failure(String),
}

struct RendererState {
    webview: Option<WebView>,
    protocol: BridgeProtocol,
    changed: Rc<Cell<bool>>,
}

/// The revision gate is deliberately independent of Wry so its semantics can
/// be exercised without a native WebView: only the newest request may change
/// visible renderer status, and a pre-attach request remains queued.
struct BridgeProtocol {
    status: RendererStatus,
    last_revision: u64,
    ready: bool,
    attached: bool,
    queued: Option<(u64, String)>,
}

impl BridgeProtocol {
    fn new() -> Self {
        Self {
            status: RendererStatus::Loading,
            last_revision: 0,
            ready: false,
            attached: false,
            queued: None,
        }
    }

    fn render(&mut self, revision: u64, script: String) -> Option<String> {
        if revision <= self.last_revision {
            return None;
        }
        self.last_revision = revision;
        self.status = RendererStatus::Loading;
        if self.ready && self.attached {
            Some(script)
        } else {
            self.queued = Some((revision, script));
            None
        }
    }

    fn bridge_ready(&mut self, attached: bool) -> Option<String> {
        self.ready = true;
        self.attached |= attached;
        self.attached
            .then(|| self.queued.take().map(|(_, script)| script))
            .flatten()
    }

    fn attached(&mut self) -> Option<String> {
        self.attached = true;
        self.ready
            .then(|| self.queued.take().map(|(_, script)| script))
            .flatten()
    }

    fn callback(&mut self, revision: Option<u64>, status: RendererStatus) {
        if revision == Some(self.last_revision) {
            self.status = status;
        }
    }

    fn failure(&mut self, error: String) {
        self.status = RendererStatus::Failure(error);
    }
}

fn renderer_html() -> Result<String, &'static str> {
    Ok(include_str!("assets/diff.html")
        .replace("__THIRD_PARTY_NOTICES__", THIRD_PARTY_NOTICES)
        .replace(
            "__PIERRE_BUNDLE__",
            include_str!("assets/pierre-diffs-bundle.js"),
        ))
}

fn allow_navigation(url: &str) -> bool {
    url == "about:blank"
}

fn render_payload(revision: u64, old: &str, new: &str, mode: &str) -> serde_json::Value {
    let line_diff_type = if contains_complex_emoji(old) || contains_complex_emoji(new) {
        "none"
    } else {
        "word-alt"
    };
    serde_json::json!({"revision":revision,"oldFile":{"name":"Comparison.txt","contents":old},"newFile":{"name":"Comparison.txt","contents":new},"options":{"theme":{"dark":"pierre-dark","light":"pierre-light"},"themeType":"dark","diffStyle":mode,"overflow":"scroll","diffIndicators":"bars","hunkSeparators":"line-info","lineDiffType":line_diff_type,"tokenizeMaxLength":500000,"tokenizeMaxLineLength":10000}})
}

fn preserve_initialization_failure(
    webview_attached: bool,
    status: &RendererStatus,
) -> Result<(), String> {
    if !webview_attached {
        if let RendererStatus::Failure(error) = status {
            return Err(error.clone());
        }
    }
    Ok(())
}

impl TextDiffRenderer {
    pub(super) fn new(window: &Window, changed: Rc<Cell<bool>>) -> Self {
        let state = Rc::new(RefCell::new(RendererState {
            webview: None,
            protocol: BridgeProtocol::new(),
            changed,
        }));
        let bridge_state = Rc::downgrade(&state);
        let html = match renderer_html() {
            Ok(html) => html,
            Err(error) => {
                {
                    let mut state_ref = state.borrow_mut();
                    state_ref.protocol.failure(error.into());
                    state_ref.changed.set(true);
                }
                return Self(state);
            }
        };
        let bounds = Rect {
            position: LogicalPosition::new(0., 0.).into(),
            size: LogicalSize::new(0., 0.).into(),
        };
        let result = WebViewBuilder::new()
            .with_bounds(bounds)
            .with_html(html)
            // The document and bundle are constructed from local checked-in
            // bytes. Reject all later navigation, including remote URLs.
            .with_navigation_handler(|url: String| allow_navigation(&url))
            // Wry enables inspector affordances in debug builds unless this is
            // explicitly disabled; this renderer never exposes web devtools.
            .with_devtools(false)
            .with_ipc_handler(move |request| {
                let Some(bridge_state) = bridge_state.upgrade() else {
                    return;
                };
                let message: serde_json::Value =
                    serde_json::from_str(request.body()).unwrap_or_default();
                let mut state = bridge_state.borrow_mut();
                match message.get("type").and_then(serde_json::Value::as_str) {
                    Some("bridgeReady") => {
                        // Wry can deliver bridgeReady while build_as_child is
                        // still attaching the native child. Retain the newest
                        // request until it can actually be submitted.
                        let attached = state.webview.is_some();
                        let script = state.protocol.bridge_ready(attached);
                        if let (Some(webview), Some(script)) = (&state.webview, script) {
                            if let Err(error) = webview.evaluate_script(&script) {
                                state.protocol.failure(error.to_string());
                            }
                        }
                    }
                    Some("ready") => {
                        state.protocol.callback(
                            message.get("revision").and_then(serde_json::Value::as_u64),
                            RendererStatus::Ready,
                        );
                    }
                    Some("error") => state.protocol.callback(
                        message.get("revision").and_then(serde_json::Value::as_u64),
                        RendererStatus::Failure(
                            message
                                .get("message")
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or("The local renderer failed.")
                                .to_owned(),
                        ),
                    ),
                    _ => {}
                }
                state.changed.set(true);
            })
            .build_as_child(window);
        match result {
            Ok(webview) => {
                let mut state_ref = state.borrow_mut();
                state_ref.webview = Some(webview);
                let queued = state_ref.protocol.attached();
                if let (Some(webview), Some(script)) = (&state_ref.webview, queued) {
                    if let Err(error) = webview.evaluate_script(&script) {
                        state_ref.protocol.failure(error.to_string());
                    }
                }
                state_ref.changed.set(true);
            }
            Err(error) => {
                let mut state = state.borrow_mut();
                state.protocol.failure(error.to_string());
                state.changed.set(true);
            }
        }
        Self(state)
    }

    pub(super) fn render(
        &self,
        revision: u64,
        old: String,
        new: String,
        mode: &str,
    ) -> Result<(), String> {
        let mut state = self.0.borrow_mut();
        preserve_initialization_failure(state.webview.is_some(), &state.protocol.status)?;
        let payload = render_payload(revision, &old, &new, mode);
        // Pierre can emit ready/error during the same JavaScript turn. Queue
        // it after evaluate_script returns so the IPC callback never re-borrows
        // this renderer while its native WebView is being submitted.
        let script = format!("setTimeout(()=>window.pierreBridge.renderDiff({payload}),0);");
        let script = state.protocol.render(revision, script);
        if let Some(script) = script {
            let Some(webview) = &state.webview else {
                return Err("The embedded renderer is unavailable.".into());
            };
            webview
                .evaluate_script(&script)
                .map_err(|error| error.to_string())?;
        }
        state.changed.set(true);
        Ok(())
    }

    pub(super) fn status(&self) -> RendererStatus {
        self.0.borrow().protocol.status.clone()
    }

    #[cfg(debug_assertions)]
    pub(super) fn render_proof_failure(&self, revision: u64) -> Result<(), String> {
        let mut state = self.0.borrow_mut();
        let script = state.protocol.render(
            revision,
            // Defer until this Rust RefCell borrow ends. WebKit may deliver the
            // bridge's caught error synchronously from evaluate_script.
            format!("setTimeout(()=>window.pierreBridge.renderDiff({{revision:{revision}}}),0);"),
        );
        if let Some(script) = script {
            let Some(webview) = &state.webview else {
                return Err("The embedded renderer is unavailable.".into());
            };
            webview
                .evaluate_script(&script)
                .map_err(|error| error.to_string())?;
        }
        state.changed.set(true);
        Ok(())
    }

    fn set_bounds(&self, bounds: Bounds<Pixels>) {
        if let Some(webview) = &self.0.borrow().webview {
            let rect = Rect {
                position: LogicalPosition::new(f64::from(bounds.left()), f64::from(bounds.top()))
                    .into(),
                size: LogicalSize::new(f64::from(bounds.size.width), f64::from(bounds.size.height))
                    .into(),
            };
            let _ = webview.set_bounds(rect);
        }
    }

    pub(super) fn set_active(&self, active: bool) {
        if let Some(webview) = &self.0.borrow().webview {
            let _ = webview.set_visible(active);
        }
    }

    pub(super) fn focus_parent(&self) {
        if let Some(webview) = &self.0.borrow().webview {
            let _ = webview.focus_parent();
        }
    }
}

fn contains_complex_emoji(text: &str) -> bool {
    text.chars().any(|character| {
        let scalar = character as u32;
        (0x1F000..=0x1FAFF).contains(&scalar) || (0x2600..=0x27BF).contains(&scalar)
    })
}

pub(super) struct WebDiffSurface {
    renderer: TextDiffRenderer,
}
impl WebDiffSurface {
    pub(super) fn new(renderer: TextDiffRenderer) -> Self {
        Self { renderer }
    }
}
impl IntoElement for WebDiffSurface {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for WebDiffSurface {
    type RequestLayoutState = ();
    type PrepaintState = ();
    fn id(&self) -> Option<ElementId> {
        Some("text-diff-web-surface".into())
    }
    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = relative(1.).into();
        (window.request_layout(style, [], cx), ())
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        _: &mut Window,
        _: &mut App,
    ) {
        self.renderer.set_bounds(bounds);
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        _: &mut Window,
        _: &mut App,
    ) {
    }
}

#[cfg(test)]
mod protocol_tests {
    use super::{
        allow_navigation, preserve_initialization_failure, render_payload, renderer_html,
        BridgeProtocol, RendererStatus,
    };

    #[test]
    fn html_uses_only_inline_local_renderer_resources_with_network_csp() {
        let html = renderer_html().expect("checked-in renderer resources are embedded");
        assert!(!html.contains("__PIERRE_BUNDLE__"));
        assert!(!html.contains("__THIRD_PARTY_NOTICES__"));
        assert!(html.contains("window.ipc"));
        assert!(html.contains("default-src 'none'"));
        assert!(html.contains("connect-src 'none'"));
        assert!(html.contains("object-src 'none'"));
        assert!(!html.contains("<script src="));
        assert!(!html.contains("<link rel=\"stylesheet\" href="));
    }

    #[test]
    fn wry_allows_only_the_initial_blank_document_navigation() {
        assert!(allow_navigation("about:blank"));
        assert!(!allow_navigation("https://example.com/"));
        assert!(!allow_navigation("file:///etc/passwd"));
        assert!(!allow_navigation("javascript:alert(1)"));
        assert!(!allow_navigation("data:text/html,remote"));
    }

    #[test]
    fn renderer_payload_round_trips_utf8_and_selects_the_disclosed_emoji_fallback() {
        let old = "caffè café 🇮🇹 👩🏽‍💻 1️⃣";
        let new = "caffè cafés 🇮🇹 👩🏽‍💻 1️⃣";
        let payload = render_payload(17, old, new, "split");
        assert_eq!(payload["oldFile"]["contents"], old);
        assert_eq!(payload["newFile"]["contents"], new);
        assert_eq!(payload["options"]["lineDiffType"], "none");
        assert_eq!(payload["revision"], 17);

        let ordinary = render_payload(18, "caffè", "café", "unified");
        assert_eq!(ordinary["options"]["lineDiffType"], "word-alt");
    }

    #[test]
    fn initialization_failure_remains_visible_when_the_first_request_arrives() {
        let error = preserve_initialization_failure(
            false,
            &RendererStatus::Failure("native child unavailable".into()),
        );
        assert_eq!(error, Err("native child unavailable".into()));
    }

    #[test]
    fn newest_request_survives_early_and_duplicate_bridge_ready() {
        let mut protocol = BridgeProtocol::new();
        protocol.render(1, "first".into());
        protocol.render(2, "newest".into());
        assert_eq!(protocol.bridge_ready(false), None);
        assert_eq!(protocol.queued, Some((2, "newest".into())));
        // A new request between bridge readiness and native attachment must
        // replace the queue instead of being submitted to a missing WebView.
        assert_eq!(protocol.render(3, "latest before attach".into()), None);
        assert_eq!(protocol.queued, Some((3, "latest before attach".into())));
        assert_eq!(
            protocol.bridge_ready(true),
            Some("latest before attach".into())
        );
        assert_eq!(protocol.attached(), None);
        assert_eq!(protocol.queued, None);
        assert_eq!(protocol.bridge_ready(true), None);
        assert_eq!(
            protocol.render(4, "attached".into()),
            Some("attached".into())
        );
    }

    #[test]
    fn stale_callbacks_cannot_change_the_current_renderer_status() {
        let mut protocol = BridgeProtocol::new();
        assert_eq!(protocol.bridge_ready(true), None);
        protocol.render(7, "current".into());
        protocol.callback(Some(6), RendererStatus::Failure("old failure".into()));
        assert_eq!(protocol.status, RendererStatus::Loading);
        protocol.callback(Some(7), RendererStatus::Failure("current failure".into()));
        assert_eq!(
            protocol.status,
            RendererStatus::Failure("current failure".into())
        );
        protocol.callback(Some(6), RendererStatus::Ready);
        assert_eq!(
            protocol.status,
            RendererStatus::Failure("current failure".into())
        );
    }

    #[test]
    fn latest_revision_can_recover_after_a_render_error() {
        let mut protocol = BridgeProtocol::new();
        protocol.bridge_ready(true);
        assert_eq!(protocol.render(4, "first".into()), Some("first".into()));
        protocol.callback(Some(4), RendererStatus::Failure("draw failed".into()));
        assert_eq!(
            protocol.status,
            RendererStatus::Failure("draw failed".into())
        );
        assert_eq!(protocol.render(5, "retry".into()), Some("retry".into()));
        assert_eq!(protocol.status, RendererStatus::Loading);
        protocol.callback(Some(5), RendererStatus::Ready);
        assert_eq!(protocol.status, RendererStatus::Ready);
    }
}
