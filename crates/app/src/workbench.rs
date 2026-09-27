//! The long-lived application shell and concrete Utility workspace sessions.

use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

use gpui::prelude::*;
use gpui::{
    div, px, AnyView, AnyWindowHandle, App, Context, Entity, FocusHandle, IntoElement, Render,
    ScrollHandle, Subscription, Window,
};
use gpui_kit::assets::IconName;
use gpui_kit::component::{
    button::{Button, ButtonVariants as _},
    input::{Input, InputEvent, InputState},
    ActiveTheme as _, Icon, Root, Sizable as _, WindowExt as _,
};

use crate::appearance::{self, AppearanceMode};
use crate::clipboard::Clipboard;
use crate::history::{HistoryRecorder, HistorySubscription};
use crate::json_workspace::JsonWorkspace;
use crate::preferences::{
    ShortcutPreferences, StartupShortcut, WorkspacePreferences, WorkspacePreferencesData,
};
use crate::registry::{OpenUtility, UtilityDefinition, UtilityId, UtilityRegistry, WorkspaceViews};
use crate::shortcut::{Shortcut, ShortcutController};
use crate::text_diff::TextDiffWorkspace;
use crate::ui;
use crate::workspace_layout::WorkspaceLayout;

#[cfg(target_os = "macos")]
use crate::shortcut::macos::CarbonShortcutRegistrar;

const MAX_RECENTS: usize = 8;

gpui::actions!(workbench_catalog, [CatalogPrevious, CatalogNext]);
const CATALOG_KEY_CONTEXT: &str = "SofDevToolCatalog";

/// A catalog scope. Recent and Favorites are catalog filters, not dashboards.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CatalogScope {
    Library,
    Recent,
    Favorites,
}

/// The GPUI Workbench retains each concrete workspace entity for the whole
/// application run. No workspace input or result is persisted here.
pub struct Workbench {
    registry: UtilityRegistry,
    selected: UtilityId,
    clipboard: Rc<dyn Clipboard>,
    history: Rc<HistoryRecorder>,
    layout: Entity<WorkspaceLayout>,
    json: Entity<JsonWorkspace>,
    json_history: AnyView,
    text_diff: Entity<TextDiffWorkspace>,
    text_diff_history: AnyView,
    others: HashMap<UtilityId, WorkspaceViews>,
    workspace_preferences: Option<WorkspacePreferences>,
    appearance: AppearanceMode,
    favorites: Vec<String>,
    recents: Vec<String>,
    scope: CatalogScope,
    search: Entity<InputState>,
    catalog_focus: Vec<FocusHandle>,
    catalog_scroll: ScrollHandle,
    launcher: Option<AnyWindowHandle>,
    history_sheet_owner: Option<UtilityId>,
    history_sheet_programmatic_close: Option<Rc<Cell<bool>>>,
    main_window: AnyWindowHandle,
    entity: gpui::WeakEntity<Self>,
    shortcut_requested: Arc<AtomicBool>,
    #[cfg(target_os = "macos")]
    shortcut: ShortcutController<CarbonShortcutRegistrar>,
    shortcut_error: Option<String>,
    shortcut_preferences: Option<ShortcutPreferences>,
    _search_subscription: Subscription,
    _appearance_subscription: Subscription,
    _layout_subscription: Subscription,
    _history_status_subscription: HistorySubscription,
}

impl Workbench {
    pub fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        clipboard: Rc<dyn Clipboard>,
        history: Rc<HistoryRecorder>,
    ) -> Self {
        cx.bind_keys([
            gpui::KeyBinding::new("up", CatalogPrevious, Some(CATALOG_KEY_CONTEXT)),
            gpui::KeyBinding::new("down", CatalogNext, Some(CATALOG_KEY_CONTEXT)),
        ]);
        let layout = cx.new(|_| WorkspaceLayout::load_from_application_support());
        let layout_subscription = cx.observe_in(&layout, window, |_this, _, window, cx| {
            cx.notify();
            cx.defer_in(window, |this, window, cx| {
                this.sync_history_sheet(window, cx)
            });
        });
        let json = cx.new(|cx| {
            JsonWorkspace::new_with_layout(
                window,
                cx,
                clipboard.clone(),
                history.clone(),
                layout.clone(),
            )
        });
        let json_history: AnyView = cx
            .new(|cx| {
                ui::HistoryInspector::new(
                    json.clone(),
                    |workspace, cx| workspace.render_history(cx).into_any_element(),
                    cx,
                )
            })
            .into();
        let text_diff = cx.new(|cx| {
            TextDiffWorkspace::new_with_layout(
                window,
                cx,
                clipboard.clone(),
                history.clone(),
                layout.clone(),
            )
        });
        let text_diff_history: AnyView = cx
            .new(|cx| {
                ui::HistoryInspector::new(
                    text_diff.clone(),
                    |workspace, cx| workspace.render_history(cx).into_any_element(),
                    cx,
                )
            })
            .into();
        text_diff.read(cx).set_active(false);

        let workspace_preferences = WorkspacePreferences::application_support().ok();
        let data = workspace_preferences
            .as_ref()
            .map(WorkspacePreferences::load)
            .unwrap_or_default();
        let appearance = AppearanceMode::from_stored(&data.theme);
        appearance::apply(appearance, Some(window), cx);

        let search = cx.new(|cx| InputState::new(window, cx));
        let search_subscription = cx.subscribe_in(
            &search,
            window,
            |_this, _entity, event: &InputEvent, _window, cx| {
                if matches!(event, InputEvent::Change) {
                    cx.notify();
                }
            },
        );
        let appearance_subscription = cx.observe_window_appearance(window, |this, window, cx| {
            if this.appearance == AppearanceMode::System {
                appearance::apply_system(window, cx);
            }
        });
        let registry = UtilityRegistry::initial();
        let weak = cx.weak_entity();
        let history_status_subscription = history.subscribe_status(move |cx| {
            let weak = weak.clone();
            cx.defer(move |cx| {
                weak.update(cx, |_this, cx| cx.notify()).ok();
            });
        });

        let shortcut_requested = Arc::new(AtomicBool::new(false));
        #[cfg(target_os = "macos")]
        let mut shortcut = ShortcutController::new(CarbonShortcutRegistrar::new({
            let shortcut_requested = shortcut_requested.clone();
            move || shortcut_requested.store(true, Ordering::Release)
        }));
        #[cfg(target_os = "macos")]
        let shortcut_error = shortcut
            .register(Shortcut::launcher_default())
            .err()
            .map(|error| error.to_string());
        let workbench = Self {
            registry,
            selected: UtilityId::Json,
            clipboard,
            history,
            layout,
            json,
            json_history,
            text_diff,
            text_diff_history,
            others: HashMap::new(),
            workspace_preferences,
            appearance,
            favorites: data.favorites,
            recents: data.recents,
            scope: CatalogScope::Library,
            search,
            catalog_focus: Vec::new(),
            catalog_scroll: ScrollHandle::new(),
            launcher: None,
            history_sheet_owner: None,
            history_sheet_programmatic_close: None,
            main_window: window.window_handle(),
            entity: cx.weak_entity(),
            shortcut_requested,
            #[cfg(target_os = "macos")]
            shortcut,
            shortcut_error,
            shortcut_preferences: None,
            _search_subscription: search_subscription,
            _appearance_subscription: appearance_subscription,
            _layout_subscription: layout_subscription,
            _history_status_subscription: history_status_subscription,
        };
        workbench.observe_global_shortcut(cx);
        workbench
    }

    pub fn open(&mut self, request: OpenUtility, cx: &mut Context<Self>) -> bool {
        let opened = self.registry.open(request.0, &mut self.selected);
        if opened {
            self.layout
                .update(cx, |layout, cx| layout.set_active(request.0, cx));
            self.record_recent(request.0);
            self.sync_workspace_visibility(cx);
            cx.notify();
        }
        opened
    }

    fn record_recent(&mut self, id: UtilityId) {
        let slug = id.slug().to_owned();
        self.recents.retain(|existing| existing != &slug);
        self.recents.insert(0, slug);
        self.recents.truncate(MAX_RECENTS);
        self.persist_workspace();
    }

    fn toggle_favorite(&mut self, id: UtilityId) {
        let slug = id.slug().to_owned();
        if let Some(index) = self.favorites.iter().position(|existing| existing == &slug) {
            self.favorites.remove(index);
        } else {
            self.favorites.push(slug);
        }
        self.persist_workspace();
    }

    fn is_favorite(&self, id: UtilityId) -> bool {
        self.favorites.iter().any(|slug| slug == id.slug())
    }

    pub(crate) fn set_appearance(
        &mut self,
        mode: AppearanceMode,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.appearance = mode;
        appearance::apply(mode, Some(window), cx);
        self.persist_workspace();
        cx.notify();
    }

    pub(crate) fn appearance_mode(&self) -> AppearanceMode {
        self.appearance
    }

    fn set_scope(&mut self, scope: CatalogScope, cx: &mut Context<Self>) {
        self.scope = scope;
        cx.notify();
    }

    fn move_catalog_focus(&mut self, delta: isize, window: &mut Window, cx: &mut Context<Self>) {
        let query = self.search.read(cx).value().to_string();
        let mut definitions = self.filtered_definitions(&query);
        if self.scope == CatalogScope::Library {
            definitions.sort_by_key(|definition| catalog_category_rank(definition.category));
        }
        if definitions.is_empty() || self.catalog_focus.len() != self.registry.definitions().len() {
            return;
        }
        let focused = window.focused(cx);
        let index = definitions
            .iter()
            .position(|definition| {
                focused.as_ref() == Some(&self.catalog_focus[self.registry_index(definition.id)])
            })
            .or_else(|| {
                definitions
                    .iter()
                    .position(|definition| definition.id == self.selected)
            })
            .unwrap_or(0) as isize;
        let next = (index + delta).rem_euclid(definitions.len() as isize) as usize;
        let focus = self.catalog_focus[self.registry_index(definitions[next].id)].clone();
        let scroll_index = if self.scope == CatalogScope::Library {
            let mut child_index = 0;
            let mut category = "";
            for definition in definitions.iter().take(next + 1) {
                if definition.category != category {
                    category = definition.category;
                    child_index += 1;
                }
                child_index += 1;
            }
            child_index - 1
        } else {
            next
        };
        self.catalog_scroll.scroll_to_item(scroll_index);
        window.focus(&focus, cx);
    }

    fn catalog_previous(
        &mut self,
        _: &CatalogPrevious,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.move_catalog_focus(-1, window, cx);
    }

    fn catalog_next(&mut self, _: &CatalogNext, window: &mut Window, cx: &mut Context<Self>) {
        self.move_catalog_focus(1, window, cx);
    }

    fn persist_workspace(&self) {
        if let Some(preferences) = &self.workspace_preferences {
            let _ = preferences.save(&WorkspacePreferencesData {
                theme: self.appearance.stored_value().to_owned(),
                favorites: self.favorites.clone(),
                recents: self.recents.clone(),
            });
        }
    }

    /// The catalog entries for the current scope, filtered by the search query.
    fn filtered_definitions(&self, query: &str) -> Vec<UtilityDefinition> {
        let matched: Vec<UtilityId> = if query.trim().is_empty() {
            self.registry.definitions().iter().map(|d| d.id).collect()
        } else {
            self.registry.search(query)
        };
        let base: Vec<UtilityDefinition> = match self.scope {
            CatalogScope::Library => self.registry.definitions().to_vec(),
            CatalogScope::Recent => self
                .recents
                .iter()
                .filter_map(|slug| self.registry.definition_by_slug(slug))
                .collect(),
            CatalogScope::Favorites => self
                .favorites
                .iter()
                .filter_map(|slug| self.registry.definition_by_slug(slug))
                .collect(),
        };
        base.into_iter()
            .filter(|definition| matched.contains(&definition.id))
            .collect()
    }

    fn registry_index(&self, id: UtilityId) -> usize {
        self.registry
            .definitions()
            .iter()
            .position(|definition| definition.id == id)
            .unwrap_or(0)
    }

    fn sync_workspace_visibility(&self, cx: &App) {
        self.text_diff.read(cx).set_active(
            self.selected == UtilityId::TextDiff
                && self.launcher.is_none()
                && self.history_sheet_owner.is_none(),
        );
    }

    fn sync_history_sheet(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let desired = (self.layout.read(cx).placement(self.selected)
            == crate::workspace_layout::HistoryPlacement::Sheet)
            .then_some(self.selected);
        if self.history_sheet_owner == desired {
            return;
        }
        if self.history_sheet_owner.take().is_some() {
            if let Some(closing) = self.history_sheet_programmatic_close.take() {
                closing.set(true);
            }
            window.close_sheet(cx);
        }
        if let Some(id) = desired {
            let history = self.workspace_views(id, window, cx).history;
            let layout = self.layout.clone();
            let weak = cx.weak_entity();
            let programmatic_close = Rc::new(Cell::new(false));
            self.history_sheet_programmatic_close = Some(programmatic_close.clone());
            self.history_sheet_owner = Some(id);
            self.sync_workspace_visibility(cx);
            window.open_sheet(cx, move |sheet, _, _| {
                let layout = layout.clone();
                let weak = weak.clone();
                let programmatic_close = programmatic_close.clone();
                sheet
                    .title("History")
                    .size(px(340.))
                    .resizable(true)
                    .child(history.clone())
                    .on_close(move |_, _, cx| {
                        if programmatic_close.get() {
                            return;
                        }
                        layout.update(cx, |layout, cx| layout.set_history_visible(id, false, cx));
                        weak.update(cx, |this, cx| {
                            this.history_sheet_owner = None;
                            this.history_sheet_programmatic_close = None;
                            this.sync_workspace_visibility(cx);
                            cx.notify();
                        })
                        .ok();
                    })
            });
        } else {
            self.sync_workspace_visibility(cx);
        }
    }

    fn show_launcher(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(launcher) = self.launcher.take() {
            if launcher
                .update(cx, |_launcher, launcher_window, _cx| {
                    launcher_window.remove_window()
                })
                .is_ok()
            {
                return;
            }
        }
        self.text_diff.read(cx).set_active(false);
        self.launcher = crate::launcher::show(
            self.registry.clone(),
            window.window_handle(),
            cx.weak_entity(),
            cx,
        );
    }

    /// Called from the application-owned shortcut observer rather than a
    /// render pass. A hidden Workbench remains a live GPUI window, so this
    /// works when the user has closed its visible native surface.
    fn consume_global_shortcut(&mut self, cx: &mut Context<Self>) {
        if !self.shortcut_requested.swap(false, Ordering::AcqRel) {
            return;
        }
        if let Some(launcher) = self.launcher.take() {
            if launcher
                .update(cx, |_launcher, launcher_window, _cx| {
                    launcher_window.remove_window()
                })
                .is_ok()
            {
                self.sync_workspace_visibility(cx);
                cx.notify();
                return;
            }
        }
        let main_window = self.main_window;
        self.text_diff.read(cx).set_active(false);
        self.launcher =
            crate::launcher::show(self.registry.clone(), main_window, self.entity.clone(), cx);
    }

    pub fn launcher_did_dismiss(&mut self, cx: &mut Context<Self>) {
        self.launcher = None;
        self.sync_workspace_visibility(cx);
        cx.notify();
    }

    fn observe_global_shortcut(&self, cx: &mut Context<Self>) {
        let requested = self.shortcut_requested.clone();
        let executor = cx.background_executor().clone();
        cx.spawn(async move |this, cx| loop {
            executor.timer(Duration::from_millis(50)).await;
            if this
                .update(cx, |_workbench, cx| {
                    if requested.load(Ordering::Acquire) {
                        _workbench.consume_global_shortcut(cx);
                    }
                })
                .is_err()
            {
                break;
            }
        })
        .detach();
    }

    pub fn configure_shortcut(&mut self, shortcut: Shortcut, cx: &mut Context<Self>) {
        #[cfg(target_os = "macos")]
        {
            if let Err(error) = self.shortcut.register(shortcut) {
                self.shortcut_error = Some(error.to_string());
            } else {
                self.shortcut_error = None;
            }
            cx.notify();
        }
    }

    pub fn restore_shortcut_preferences(
        &mut self,
        preferences: Option<ShortcutPreferences>,
        startup: StartupShortcut,
        cx: &mut Context<Self>,
    ) {
        self.shortcut_preferences = preferences;
        let mut diagnostics = Vec::new();
        if let Some(shortcut) = startup.shortcut {
            #[cfg(target_os = "macos")]
            if let Err(error) = self.shortcut.register(shortcut) {
                diagnostics.push(error.to_string());
            }
        } else if let Some(error) = self.shortcut_error.take() {
            diagnostics.push(error);
        }
        diagnostics.extend(startup.diagnostic);
        // A failed default registration remains visible when no saved shortcut
        // replaces it. A successfully registered saved shortcut clears that
        // stale default error instead of presenting an unreachable state.
        self.shortcut_error = (!diagnostics.is_empty()).then(|| diagnostics.join(" "));
        cx.notify();
    }

    fn show_settings(&mut self, cx: &mut Context<Self>) {
        if let Some(preferences) = self.shortcut_preferences.clone() {
            let utilities = self
                .registry
                .definitions()
                .iter()
                .map(|definition| crate::settings::UtilityRow {
                    id: definition.id.slug().to_owned(),
                    name: definition.name.to_owned(),
                    default_enabled: definition.history_enabled_by_default,
                })
                .collect();
            crate::settings::show(
                cx.weak_entity(),
                preferences,
                self.history.clone(),
                utilities,
                self.current_shortcut(),
                self.appearance_mode(),
                cx,
            );
        } else {
            self.shortcut_error = Some(
                "Launcher settings are unavailable because the Rust Application Support directory could not be resolved."
                    .into(),
            );
            cx.notify();
        }
    }

    /// The committed OS shortcut. Settings persists only this value after the
    /// native registrar accepts a candidate.
    pub fn current_shortcut(&self) -> Shortcut {
        #[cfg(target_os = "macos")]
        {
            self.shortcut.shortcut()
        }
        #[cfg(not(target_os = "macos"))]
        {
            Shortcut::launcher_default()
        }
    }

    /// Returns the selected Utility's view, constructing it once on first open.
    fn workspace_views(
        &mut self,
        id: UtilityId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> WorkspaceViews {
        match id {
            UtilityId::Json => WorkspaceViews {
                body: self.json.clone().into(),
                history: self.json_history.clone(),
            },
            UtilityId::TextDiff => WorkspaceViews {
                body: self.text_diff.clone().into(),
                history: self.text_diff_history.clone(),
            },
            other => {
                if let Some(views) = self.others.get(&other) {
                    return views.clone();
                }
                let construct = self
                    .registry
                    .definition(other)
                    .and_then(|definition| definition.construct)
                    .expect("a catalog Utility exposes a workspace constructor");
                let views = construct(
                    window,
                    cx,
                    self.clipboard.clone(),
                    self.history.clone(),
                    self.layout.clone(),
                );
                self.others.insert(other, views.clone());
                views
            }
        }
    }
}

fn catalog_category_rank(category: &str) -> usize {
    match category {
        "Format & Convert" => 0,
        "Inspect" => 1,
        "Generate" => 2,
        "Text" => 3,
        _ => 4,
    }
}

fn catalog_category_heading(category: &str) -> &str {
    match category {
        "Inspect" => "ENCODE & INSPECT",
        "Text" => "COMPARE & TEST",
        "Format & Convert" => "FORMAT & CONVERT",
        "Generate" => "GENERATE",
        _ => category,
    }
}

impl Render for Workbench {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let workspace_width = window.bounds().size.width - px(228.);
        let crossed_history_breakpoint = self.layout.update(cx, |layout, cx| {
            layout.set_workspace_width(workspace_width, cx)
        });
        if crossed_history_breakpoint {
            cx.defer_in(window, |this, window, cx| {
                this.sync_history_sheet(window, cx);
            });
        }
        let selected = self.selected;
        let scope = self.scope;
        let query = self.search.read(cx).value().to_string();
        let mut definitions = self.filtered_definitions(&query);
        if scope == CatalogScope::Library {
            // Preserve source metadata and within-category order while giving
            // the approved grouped catalog one heading per category.
            definitions.sort_by_key(|definition| catalog_category_rank(definition.category));
        }

        // Kit buttons own their focus handle, keyed by element id. Rebuild the
        // catalog handles with the window and view in scope so arrow-key
        // navigation can address the same handles.
        if self.catalog_focus.len() != self.registry.definitions().len() {
            self.catalog_focus = self
                .registry
                .definitions()
                .iter()
                .map(|definition| {
                    ui::button_focus(window, format!("catalog-{}", definition.id.slug()), cx)
                })
                .collect();
        }

        let scope_button =
            |label: &'static str, target: CatalogScope, index: usize, cx: &mut Context<Self>| {
                Button::new(format!("catalog.scope.{index}"))
                    .label(label)
                    .xsmall()
                    .ghost()
                    .when(scope == target, |button| button.primary())
                    .on_click(cx.listener(move |this, _event, _window, cx| {
                        this.set_scope(target, cx);
                    }))
            };

        let mut catalog = div()
            .id("catalog-list")
            .key_context(CATALOG_KEY_CONTEXT)
            .on_action(cx.listener(Self::catalog_previous))
            .on_action(cx.listener(Self::catalog_next))
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .gap_1()
            .track_scroll(&self.catalog_scroll)
            .overflow_y_scroll();
        let mut current_category = "";
        for definition in &definitions {
            if scope == CatalogScope::Library && definition.category != current_category {
                current_category = definition.category;
                catalog = catalog.child(
                    div()
                        .mt_3()
                        .mb_1()
                        .px_1()
                        .text_xs()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.muted_foreground)
                        .child(catalog_category_heading(current_category).to_owned()),
                );
            }
            let id = definition.id;
            let favorite = self.is_favorite(id);
            catalog = catalog.child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .w_full()
                    .h(px(32.))
                    .rounded(theme.radius)
                    .when(id == selected, |row| row.bg(theme.secondary))
                    .child(
                        div().flex_1().min_w_0().child(
                            Button::new(format!("catalog-{}", id.slug()))
                                .ghost()
                                .xsmall()
                                .w_full()
                                .accessibility_label(format!("Open {}", definition.name))
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_2()
                                        .w_full()
                                        .min_w_0()
                                        .child(ui::utility_icon(id))
                                        .child(div().min_w_0().child(definition.name)),
                                )
                                .on_click(cx.listener(move |this, _event, _window, cx| {
                                    this.open(OpenUtility(id), cx);
                                })),
                        ),
                    )
                    .child(
                        Button::new(format!("favorite-{}", id.slug()))
                            .icon(if favorite {
                                IconName::StarFill
                            } else {
                                IconName::Star
                            })
                            .ghost()
                            .xsmall()
                            .tooltip(if favorite {
                                "Remove favorite"
                            } else {
                                "Add favorite"
                            })
                            .accessibility_label(format!(
                                "{} favorite {}",
                                if favorite { "Remove" } else { "Add" },
                                definition.name
                            ))
                            .on_click(cx.listener(move |this, _event, _window, cx| {
                                this.toggle_favorite(id);
                                cx.notify();
                            })),
                    ),
            );
        }
        if definitions.is_empty() {
            catalog = catalog.child(
                div()
                    .p_3()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(match scope {
                        CatalogScope::Recent => "No recently opened Utilities yet.",
                        CatalogScope::Favorites => "No favorites yet.",
                        CatalogScope::Library => "No Utilities match the search.",
                    }),
            );
        }

        let sidebar = div()
            .flex()
            .flex_col()
            .w(px(228.))
            .flex_shrink_0()
            .min_h_0()
            .p_2()
            .gap_2()
            .bg(theme.popover)
            .border_r_1()
            .border_color(theme.border)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_1()
                    .child(scope_button("Library", CatalogScope::Library, 0, cx))
                    .child(scope_button("Recent", CatalogScope::Recent, 1, cx))
                    .child(scope_button("Favorites", CatalogScope::Favorites, 2, cx)),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child("SEARCH UTILITIES"),
                    )
                    .child(
                        Input::new(&self.search)
                            .accessibility_id("catalog.search")
                            .w_full(),
                    ),
            )
            .child(catalog)
            .child(
                div()
                    .pt_2()
                    .border_t_1()
                    .border_color(theme.border)
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(format!(
                        "{} of {} Utilities",
                        definitions.len(),
                        self.registry.definitions().len()
                    )),
            );

        let definition = self
            .registry
            .definition(selected)
            .expect("the selected Utility is registered");
        let view = self.workspace_views(selected, window, cx).body;
        let history_visible = self.layout.read(cx).history_visible(selected);
        let layout_error = self.layout.read(cx).save_error().map(str::to_owned);
        let recording = self.history.recording_state(selected.slug());
        let topbar = div()
            .flex()
            .items_center()
            .justify_between()
            .gap_3()
            .h(px(40.))
            .flex_shrink_0()
            .px_4()
            .border_b_1()
            .border_color(theme.border)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::BOLD)
                            .child(crate::identity::APP_DISPLAY_NAME),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child("Local"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        Button::new("workbench.launcher")
                            .icon(IconName::Command)
                            .ghost()
                            .tooltip(format!(
                                "Utility Launcher · {}",
                                self.current_shortcut().display_name
                            ))
                            .accessibility_label("Open Utility Launcher")
                            .on_click(cx.listener(|this, _event, window, cx| {
                                this.show_launcher(window, cx);
                            })),
                    )
                    .child(
                        Button::new("workbench.settings")
                            .icon(IconName::Settings)
                            .ghost()
                            .tooltip("Settings")
                            .accessibility_label("Open Settings")
                            .on_click(cx.listener(|this, _event, _window, cx| {
                                this.show_settings(cx);
                            })),
                    ),
            );

        let heading = div()
            .flex()
            .items_center()
            .justify_between()
            .gap_3()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(Icon::new(ui::utility_icon(selected)))
                    .child(
                        div()
                            .text_lg()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child(definition.name),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(recording.label()),
                    )
                    .child(
                        Button::new("workbench.history.toggle")
                            .icon(IconName::ClockArrowUp)
                            .ghost()
                            .tooltip(if history_visible {
                                "Hide History"
                            } else {
                                "Show History"
                            })
                            .accessibility_label(if history_visible {
                                "Hide History"
                            } else {
                                "Show History"
                            })
                            .on_click(cx.listener(|this, _event, _window, cx| {
                                let selected = this.selected;
                                this.layout.update(cx, |layout, cx| {
                                    layout.set_history_visible(
                                        selected,
                                        !layout.history_visible(selected),
                                        cx,
                                    )
                                });
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("toggle-favorite")
                            .icon(if self.is_favorite(selected) {
                                IconName::StarFill
                            } else {
                                IconName::Star
                            })
                            .ghost()
                            .tooltip(if self.is_favorite(selected) {
                                "Remove favorite"
                            } else {
                                "Add favorite"
                            })
                            .accessibility_label(if self.is_favorite(selected) {
                                "Remove favorite"
                            } else {
                                "Add favorite"
                            })
                            .on_click(cx.listener(|this, _event, _window, cx| {
                                let selected = this.selected;
                                this.toggle_favorite(selected);
                                cx.notify();
                            })),
                    ),
            );
        let workspace = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .p_4()
            .gap_3()
            .child(heading)
            .child(div().h(px(1.)).bg(theme.border))
            .child(div().flex().flex_1().min_h_0().min_w_0().child(view));
        let footer = div()
            .flex()
            .items_center()
            .justify_between()
            .gap_3()
            .h(px(37.))
            .flex_shrink_0()
            .px_4()
            .border_t_1()
            .border_color(theme.border)
            .text_xs()
            .text_color(theme.muted_foreground)
            .child(definition.category)
            .child(format!("v{}", crate::identity::VERSION));

        div()
            .flex()
            .flex_col()
            .relative()
            .size_full()
            .bg(theme.background)
            .text_color(theme.foreground)
            .text_size(px(13.))
            .child(topbar)
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .child(sidebar)
                    .child(workspace),
            )
            .when_some(self.shortcut_error.as_ref(), |this, error| {
                this.child(
                    div()
                        .px_4()
                        .py_2()
                        .text_xs()
                        .text_color(theme.danger)
                        .child(format!("Launcher: {error}")),
                )
            })
            .when_some(layout_error, |this, error| {
                this.child(
                    div()
                        .px_4()
                        .py_2()
                        .text_xs()
                        .text_color(theme.danger)
                        .child(error),
                )
            })
            .child(footer)
            .children(Root::render_sheet_layer(window, cx))
            // GPUI Kit renders modal dialogs in a separate layer that the host
            // view must include; `Root::render` does not mount it. Utility
            // restore confirmations open here.
            .children(Root::render_dialog_layer(window, cx))
    }
}
