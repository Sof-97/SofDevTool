//! The long-lived application shell and concrete Utility workspace sessions.

use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

use gpui::prelude::*;
use gpui::{
    div, AnyView, AnyWindowHandle, App, Context, Entity, FocusHandle, IntoElement, Render,
    Subscription, Window,
};
use sofdevtool_ui::{
    apply_theme, panel, view_click, Button, ButtonVariant, TextField, ThemeTokens, ThemeVariant,
};

use crate::clipboard::Clipboard;
use crate::history::HistoryRecorder;
use crate::json_workspace::JsonWorkspace;
use crate::preferences::{
    ShortcutPreferences, StartupShortcut, WorkspacePreferences, WorkspacePreferencesData,
};
use crate::registry::{OpenUtility, UtilityDefinition, UtilityId, UtilityRegistry};
use crate::shortcut::{Shortcut, ShortcutController};
use crate::text_diff::TextDiffWorkspace;

#[cfg(target_os = "macos")]
use crate::shortcut::macos::CarbonShortcutRegistrar;

const MAX_RECENTS: usize = 8;

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
    json: Entity<JsonWorkspace>,
    text_diff: Entity<TextDiffWorkspace>,
    others: HashMap<UtilityId, AnyView>,
    workspace_preferences: Option<WorkspacePreferences>,
    theme: ThemeVariant,
    favorites: Vec<String>,
    recents: Vec<String>,
    scope: CatalogScope,
    search: TextField,
    launcher_focus: FocusHandle,
    scope_focus: [FocusHandle; 3],
    theme_focus: FocusHandle,
    favorite_focus: FocusHandle,
    catalog_focus: Vec<FocusHandle>,
    launcher: Option<AnyWindowHandle>,
    main_window: AnyWindowHandle,
    entity: gpui::WeakEntity<Self>,
    shortcut_requested: Arc<AtomicBool>,
    #[cfg(target_os = "macos")]
    shortcut: ShortcutController<CarbonShortcutRegistrar>,
    shortcut_error: Option<String>,
    shortcut_preferences: Option<ShortcutPreferences>,
    _search_subscription: Subscription,
}

impl Workbench {
    pub fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        clipboard: Rc<dyn Clipboard>,
        history: Rc<HistoryRecorder>,
    ) -> Self {
        let json = cx.new(|cx| JsonWorkspace::new(window, cx, clipboard.clone(), history.clone()));
        let text_diff =
            cx.new(|cx| TextDiffWorkspace::new(window, cx, clipboard.clone(), history.clone()));
        text_diff.read(cx).set_active(false);

        let workspace_preferences = WorkspacePreferences::application_support().ok();
        let data = workspace_preferences
            .as_ref()
            .map(WorkspacePreferences::load)
            .unwrap_or_default();
        let theme = if data.theme == "catppuccin" {
            ThemeVariant::CatppuccinFrappe
        } else {
            ThemeVariant::Graphite
        };
        apply_theme(theme, cx);

        let search = TextField::new(window, cx);
        let search_subscription = search.on_change_in(window, cx, |_this, _window, cx| {
            cx.notify();
        });
        let registry = UtilityRegistry::initial();
        let catalog_focus = registry
            .definitions()
            .iter()
            .map(|_| cx.focus_handle().tab_stop(true).tab_index(0))
            .collect();

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
            json,
            text_diff,
            others: HashMap::new(),
            workspace_preferences,
            theme,
            favorites: data.favorites,
            recents: data.recents,
            scope: CatalogScope::Library,
            search,
            launcher_focus: cx.focus_handle().tab_stop(true).tab_index(0),
            scope_focus: std::array::from_fn(|_| cx.focus_handle().tab_stop(true).tab_index(0)),
            theme_focus: cx.focus_handle().tab_stop(true).tab_index(0),
            favorite_focus: cx.focus_handle().tab_stop(true).tab_index(0),
            catalog_focus,
            launcher: None,
            main_window: window.window_handle(),
            entity: cx.weak_entity(),
            shortcut_requested,
            #[cfg(target_os = "macos")]
            shortcut,
            shortcut_error,
            shortcut_preferences: None,
            _search_subscription: search_subscription,
        };
        workbench.observe_global_shortcut(cx);
        workbench
    }

    pub fn open(&mut self, request: OpenUtility, cx: &mut Context<Self>) -> bool {
        let opened = self.registry.open(request.0, &mut self.selected);
        if opened {
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

    fn toggle_theme(&mut self, cx: &mut Context<Self>) {
        self.theme = match self.theme {
            ThemeVariant::Graphite => ThemeVariant::CatppuccinFrappe,
            ThemeVariant::CatppuccinFrappe => ThemeVariant::Graphite,
        };
        apply_theme(self.theme, cx);
        self.persist_workspace();
        cx.notify();
    }

    fn set_scope(&mut self, scope: CatalogScope, cx: &mut Context<Self>) {
        self.scope = scope;
        cx.notify();
    }

    fn persist_workspace(&self) {
        if let Some(preferences) = &self.workspace_preferences {
            let _ = preferences.save(&WorkspacePreferencesData {
                theme: match self.theme {
                    ThemeVariant::Graphite => "graphite".to_owned(),
                    ThemeVariant::CatppuccinFrappe => "catppuccin".to_owned(),
                },
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
        self.text_diff
            .read(cx)
            .set_active(self.selected == UtilityId::TextDiff && self.launcher.is_none());
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
    fn workspace_view(
        &mut self,
        id: UtilityId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyView {
        match id {
            UtilityId::Json => self.json.clone().into(),
            UtilityId::TextDiff => self.text_diff.clone().into(),
            other => {
                if let Some(view) = self.others.get(&other) {
                    return view.clone();
                }
                let construct = self
                    .registry
                    .definition(other)
                    .and_then(|definition| definition.construct)
                    .expect("a catalog Utility exposes a workspace constructor");
                let view = construct(window, cx, self.clipboard.clone(), self.history.clone());
                self.others.insert(other, view.clone());
                view
            }
        }
    }
}

impl Render for Workbench {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let tokens = ThemeTokens::active();
        let selected = self.selected;
        let scope = self.scope;
        let query = self.search.text(cx);
        let definitions = self.filtered_definitions(&query);

        let scope_button =
            |label: &'static str, target: CatalogScope, index: usize, cx: &mut Context<Self>| {
                Button::new(label)
                    .variant(if scope == target {
                        ButtonVariant::Primary
                    } else {
                        ButtonVariant::Secondary
                    })
                    .focus_handle(self.scope_focus[index].clone())
                    .on_click(view_click(cx, move |this, _window, cx| {
                        this.set_scope(target, cx);
                    }))
            };

        let mut sidebar = div()
            .flex()
            .flex_col()
            .w_64()
            .min_h_0()
            .p_3()
            .gap_2()
            .border_r_1()
            .border_color(tokens.border())
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_1()
                    .child(scope_button("Library", CatalogScope::Library, 0, cx))
                    .child(scope_button("Recent", CatalogScope::Recent, 1, cx))
                    .child(scope_button("Favorites", CatalogScope::Favorites, 2, cx)),
            )
            .child(self.search.render("catalog.search"));

        let mut catalog = div()
            .id("catalog-list")
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .gap_1()
            .overflow_y_scroll();
        let mut current_category = "";
        for definition in &definitions {
            if scope == CatalogScope::Library && definition.category != current_category {
                current_category = definition.category;
                catalog = catalog.child(
                    div()
                        .mt_2()
                        .text_xs()
                        .text_color(tokens.text_muted())
                        .child(current_category),
                );
            }
            let id = definition.id;
            let registry_index = self.registry_index(id);
            let favorite = self.is_favorite(id);
            catalog = catalog.child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_1()
                    .child(
                        div().flex_1().min_w_0().child(
                            Button::new(definition.name)
                                .id(gpui::ElementId::Name(
                                    format!("catalog-{}", id.slug()).into(),
                                ))
                                .variant(if id == selected {
                                    ButtonVariant::Primary
                                } else {
                                    ButtonVariant::Secondary
                                })
                                .focus_handle(self.catalog_focus[registry_index].clone())
                                .on_click(view_click(cx, move |this, _window, cx| {
                                    this.open(OpenUtility(id), cx);
                                })),
                        ),
                    )
                    .child(
                        Button::new(if favorite { "★" } else { "☆" })
                            .id(gpui::ElementId::Name(
                                format!("favorite-{}", id.slug()).into(),
                            ))
                            .on_click(view_click(cx, move |this, _window, cx| {
                                this.toggle_favorite(id);
                                cx.notify();
                            })),
                    ),
            );
        }
        if definitions.is_empty() {
            catalog = catalog.child(
                div()
                    .mt_3()
                    .text_xs()
                    .text_color(tokens.text_muted())
                    .child(match scope {
                        CatalogScope::Recent => "No recently opened Utilities yet.",
                        CatalogScope::Favorites => "No favorites yet.",
                        CatalogScope::Library => "No Utilities match the search.",
                    }),
            );
        }
        sidebar = sidebar.child(catalog);

        let definition = self
            .registry
            .definition(selected)
            .expect("the selected Utility is registered");
        let view = self.workspace_view(selected, window, cx);

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(tokens.background())
            .text_color(tokens.text())
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_4()
                    .py_3()
                    .border_b_1()
                    .border_color(tokens.border())
                    .child(
                        div()
                            .text_lg()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child(crate::identity::APP_DISPLAY_NAME),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .text_xs()
                            .text_color(tokens.text_muted())
                            .child(format!(
                                "{} · {} · {} · {}",
                                crate::identity::VERSION,
                                crate::identity::PROFILE.channel(),
                                crate::identity::REVISION
                                    .chars()
                                    .take(8)
                                    .collect::<String>(),
                                crate::identity::SOURCE_STATE,
                            ))
                            .child(
                                Button::new(if self.is_favorite(selected) {
                                    "Favorite: on"
                                } else {
                                    "Favorite: off"
                                })
                                .id("toggle-favorite")
                                .focus_handle(self.favorite_focus.clone())
                                .on_click(view_click(
                                    cx,
                                    |this, _window, cx| {
                                        let selected = this.selected;
                                        this.toggle_favorite(selected);
                                        cx.notify();
                                    },
                                )),
                            )
                            .child(
                                Button::new(format!("Theme: {}", self.theme.label()))
                                    .id("toggle-theme")
                                    .focus_handle(self.theme_focus.clone())
                                    .on_click(view_click(cx, |this, _window, cx| {
                                        this.toggle_theme(cx);
                                    })),
                            )
                            .child(
                                Button::new("Settings")
                                    .variant(ButtonVariant::Secondary)
                                    .on_click(view_click(cx, |this, _window, cx| {
                                        this.show_settings(cx)
                                    })),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_1()
                    .min_h_0()
                    .child(sidebar)
                    .child(div().m_3().flex().flex_1().min_w_0().min_h_0().child(panel(
                        definition.name,
                        "persistent session",
                        div().flex().flex_1().min_h_0().child(view),
                    ))),
            )
            .child(
                div()
                    .flex()
                    .justify_end()
                    .px_4()
                    .py_3()
                    .border_t_1()
                    .border_color(tokens.border())
                    .child(
                        Button::new("Open Utility Launcher")
                            .focus_handle(self.launcher_focus.clone())
                            .on_click(view_click(cx, |this, window, cx| {
                                this.show_launcher(window, cx);
                            })),
                    ),
            )
            .when_some(self.shortcut_error.as_ref(), |this, error| {
                this.child(
                    div()
                        .px_4()
                        .py_2()
                        .text_xs()
                        .text_color(tokens.danger())
                        .child(format!("Launcher: {error}")),
                )
            })
    }
}
