use std::cell::Cell;
use std::rc::Rc;

use gpui::prelude::*;
use gpui::{Context, Entity, IntoElement, Render, Subscription, Window};
use sofui::{apply_custom_theme, apply_theme, TextEditor, ThemePalette, ThemeTokens, ThemeVariant};

struct ThemeProbe {
    editor: TextEditor,
    observed: Rc<Cell<ThemePalette>>,
    render_count: Rc<Cell<usize>>,
    change_count: usize,
    _subscription: Subscription,
}

impl ThemeProbe {
    fn new(
        text: &'static str,
        observed: Rc<Cell<ThemePalette>>,
        render_count: Rc<Cell<usize>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let editor = TextEditor::new(window, cx);
        editor.assign_text(text, window, cx);
        let subscription = editor.on_change_in(window, cx, |this, _, cx| {
            this.change_count += 1;
            cx.notify();
        });
        Self {
            editor,
            observed,
            render_count,
            change_count: 0,
            _subscription: subscription,
        }
    }
}

impl Render for ThemeProbe {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let tokens = ThemeTokens::active();
        self.observed.set(tokens.palette());
        self.render_count.set(self.render_count.get() + 1);
        gpui::div()
            .size_full()
            .bg(tokens.background())
            .text_color(tokens.text())
            .child(self.editor.render(false, "theme.editor"))
    }
}

#[gpui::test]
fn preset_and_custom_themes_update_two_retained_windows_and_editor_colors(
    cx: &mut gpui::TestAppContext,
) {
    cx.update(sofui::init);
    cx.update(|cx| apply_theme(ThemeVariant::Graphite, cx));

    let first_observed = Rc::new(Cell::new(ThemePalette::GRAPHITE));
    let second_observed = Rc::new(Cell::new(ThemePalette::GRAPHITE));
    let first_renders = Rc::new(Cell::new(0));
    let second_renders = Rc::new(Cell::new(0));
    let mut first_probe: Option<Entity<ThemeProbe>> = None;
    let mut second_probe: Option<Entity<ThemeProbe>> = None;
    let first = cx.add_window(|window, cx| {
        let probe = cx.new(|cx| {
            ThemeProbe::new(
                "first 👨‍👩‍👧‍👦",
                first_observed.clone(),
                first_renders.clone(),
                window,
                cx,
            )
        });
        first_probe = Some(probe.clone());
        gpui_component::Root::new(probe, window, cx)
    });
    let second = cx.add_window(|window, cx| {
        let probe = cx.new(|cx| {
            ThemeProbe::new(
                "second e\u{301}",
                second_observed.clone(),
                second_renders.clone(),
                window,
                cx,
            )
        });
        second_probe = Some(probe.clone());
        gpui_component::Root::new(probe, window, cx)
    });
    let first_probe = first_probe.unwrap();
    let second_probe = second_probe.unwrap();
    let first_window = first.into();
    let second_window = second.into();

    cx.update_window(first_window, |_, window, cx| {
        first_probe.update(cx, |probe, cx| probe.editor.focus(window, cx));
        window.draw(cx).clear(cx);
    })
    .unwrap();
    cx.update_window(second_window, |_, window, cx| window.draw(cx).clear(cx))
        .unwrap();
    let initial_renders = (first_renders.get(), second_renders.get());

    cx.update(|cx| apply_theme(ThemeVariant::CatppuccinFrappe, cx));
    cx.update_window(first_window, |_, window, cx| window.draw(cx).clear(cx))
        .unwrap();
    cx.update_window(second_window, |_, window, cx| window.draw(cx).clear(cx))
        .unwrap();
    assert_eq!(first_observed.get(), ThemePalette::CATPPUCCIN_FRAPPE);
    assert_eq!(second_observed.get(), ThemePalette::CATPPUCCIN_FRAPPE);
    assert!(first_renders.get() > initial_renders.0);
    assert!(second_renders.get() > initial_renders.1);
    assert_eq!(
        cx.read(|cx| gpui_component::Theme::global(cx).input),
        ThemeTokens::active().surface_raised()
    );
    assert_eq!(
        cx.read(|cx| gpui_component::Theme::global(cx)
            .highlight_theme
            .style
            .editor_background),
        Some(ThemeTokens::active().surface_raised())
    );
    assert_eq!(
        cx.read(|cx| gpui_component::Theme::global(cx).caret),
        ThemeTokens::active().accent()
    );

    let custom = ThemePalette {
        background: 0x171c1a,
        surface: 0x202a26,
        surface_raised: 0x2a3831,
        border: 0x52665a,
        text: 0xf0f4ed,
        text_muted: 0xb4c3b7,
        accent: 0x8ae0b0,
        accent_text: 0x102218,
        danger: 0xff9a9e,
        warning: 0xf5ca79,
    };
    cx.update(|cx| apply_custom_theme(ThemeTokens::from_palette(custom), cx));
    cx.update_window(first_window, |_, window, cx| window.draw(cx).clear(cx))
        .unwrap();
    cx.update_window(second_window, |_, window, cx| window.draw(cx).clear(cx))
        .unwrap();
    assert_eq!(first_observed.get(), custom);
    assert_eq!(second_observed.get(), custom);
    assert_eq!(
        cx.read(|cx| gpui_component::Theme::global(cx).foreground),
        ThemeTokens::from_palette(custom).text()
    );
    assert_eq!(
        cx.read(|cx| gpui_component::Theme::global(cx)
            .highlight_theme
            .style
            .editor_background),
        Some(ThemeTokens::from_palette(custom).surface_raised())
    );
    assert_eq!(
        cx.read(|cx| gpui_component::Theme::global(cx).ring),
        ThemeTokens::from_palette(custom).accent()
    );

    // A window and editor created after customization inherit the palette.
    let third_observed = Rc::new(Cell::new(ThemePalette::GRAPHITE));
    let third_renders = Rc::new(Cell::new(0));
    let mut third_probe: Option<Entity<ThemeProbe>> = None;
    let third = cx.add_window(|window, cx| {
        let probe = cx.new(|cx| {
            ThemeProbe::new(
                "third after theme",
                third_observed.clone(),
                third_renders.clone(),
                window,
                cx,
            )
        });
        third_probe = Some(probe.clone());
        gpui_component::Root::new(probe, window, cx)
    });
    let third_probe = third_probe.unwrap();
    let third_window = third.into();
    cx.update_window(third_window, |_, window, cx| window.draw(cx).clear(cx))
        .unwrap();
    assert_eq!(third_observed.get(), custom);
    assert_eq!(
        third_probe.read_with(cx, |probe, cx| probe.editor.text(cx)),
        "third after theme"
    );
    assert_eq!(
        cx.read(|cx| gpui_component::Theme::global(cx)
            .highlight_theme
            .style
            .editor_background),
        Some(ThemeTokens::from_palette(custom).surface_raised())
    );

    cx.update_window(first_window, |_, window, cx| {
        assert!(first_probe.read(cx).editor.text(cx).contains("👨‍👩‍👧‍👦"));
        assert!(first_probe.read(cx).editor.text(cx).starts_with("first"));
        assert_eq!(first_probe.read(cx).change_count, 0);
        assert!(first_probe.read(cx).editor.is_focused(window, cx));
    })
    .unwrap();
    assert_eq!(
        second_probe.read_with(cx, |probe, cx| probe.editor.text(cx)),
        "second e\u{301}"
    );
    assert_eq!(second_probe.read_with(cx, |probe, _| probe.change_count), 0);
    cx.update(|cx| apply_theme(ThemeVariant::Graphite, cx));
    cx.update_window(third_window, |_, window, cx| window.draw(cx).clear(cx))
        .unwrap();
    assert_eq!(third_observed.get(), ThemePalette::GRAPHITE);
}
