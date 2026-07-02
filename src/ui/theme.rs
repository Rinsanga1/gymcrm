use eframe::egui::{self, Color32, CornerRadius, FontFamily, FontId, Stroke, TextStyle};

/// Linear-inspired accent (indigo). Used for focus / selection highlight only.
pub const ACCENT: Color32 = Color32::from_rgb(94, 106, 210);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Light,
    Dark,
}

impl Mode {
    pub fn from_str(s: &str) -> Mode {
        if s.eq_ignore_ascii_case("light") {
            Mode::Light
        } else {
            Mode::Dark
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Mode::Light => "light",
            Mode::Dark => "dark",
        }
    }
}

struct Palette {
    dark_mode: bool,
    main: Color32,
    sidebar: Color32,
    card: Color32,
    faint: Color32,
    border: Color32,
    border_strong: Color32,
    text: Color32,
    text_muted: Color32,
    btn: Color32,
    btn_hover: Color32,
    btn_active: Color32,
    nav_selected: Color32,
}

fn light() -> Palette {
    Palette {
        dark_mode: false,
        main: Color32::from_rgb(255, 255, 255),
        sidebar: Color32::from_rgb(247, 247, 248),
        card: Color32::from_rgb(255, 255, 255),
        faint: Color32::from_rgb(250, 250, 251),
        border: Color32::from_rgb(233, 233, 236),
        border_strong: Color32::from_rgb(218, 218, 222),
        text: Color32::from_rgb(40, 41, 46),
        text_muted: Color32::from_rgb(138, 143, 152),
        btn: Color32::from_rgb(245, 245, 246),
        btn_hover: Color32::from_rgb(236, 236, 239),
        btn_active: Color32::from_rgb(228, 228, 232),
        nav_selected: Color32::from_rgb(236, 236, 239),
    }
}

fn dark() -> Palette {
    Palette {
        dark_mode: true,
        main: Color32::from_rgb(15, 15, 18),
        sidebar: Color32::from_rgb(22, 22, 26),
        card: Color32::from_rgb(24, 24, 28),
        faint: Color32::from_rgb(20, 20, 24),
        border: Color32::from_rgb(40, 40, 46),
        border_strong: Color32::from_rgb(54, 54, 60),
        text: Color32::from_rgb(227, 227, 230),
        text_muted: Color32::from_rgb(136, 139, 148),
        btn: Color32::from_rgb(30, 30, 35),
        btn_hover: Color32::from_rgb(38, 38, 44),
        btn_active: Color32::from_rgb(46, 46, 52),
        nav_selected: Color32::from_rgb(36, 36, 42),
    }
}

/// Apply the theme to the whole context. Idempotent.
pub fn apply(ctx: &egui::Context, mode: Mode) {
    let p = match mode {
        Mode::Light => light(),
        Mode::Dark => dark(),
    };
    let mut style = (*ctx.global_style()).clone();

    style.text_styles = [
        (TextStyle::Heading, FontId::new(19.0, FontFamily::Proportional)),
        (TextStyle::Body, FontId::new(13.5, FontFamily::Proportional)),
        (TextStyle::Button, FontId::new(13.0, FontFamily::Proportional)),
        (TextStyle::Small, FontId::new(11.0, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(12.5, FontFamily::Monospace)),
    ]
    .into();

    let s = &mut style.spacing;
    s.item_spacing = egui::vec2(8.0, 8.0);
    s.button_padding = egui::vec2(10.0, 5.0);
    s.menu_margin = egui::Margin::same(6);
    s.window_margin = egui::Margin::same(16);
    s.interact_size.y = 26.0;
    s.scroll.bar_width = 8.0;

    let mut v = if p.dark_mode {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    v.dark_mode = p.dark_mode;
    v.override_text_color = Some(p.text);
    v.panel_fill = p.main;
    v.window_fill = p.card;
    v.extreme_bg_color = p.card;
    v.faint_bg_color = p.faint;
    v.window_stroke = Stroke::new(1.0, p.border);
    v.window_corner_radius = CornerRadius::same(12);
    v.menu_corner_radius = CornerRadius::same(8);
    v.hyperlink_color = ACCENT;
    v.warn_fg_color = Color32::from_rgb(200, 130, 50);
    v.error_fg_color = Color32::from_rgb(210, 90, 90);

    v.selection.bg_fill = ACCENT.gamma_multiply(if p.dark_mode { 0.45 } else { 0.22 });
    v.selection.stroke = Stroke::new(1.0, ACCENT);

    let r = CornerRadius::same(7);
    let w = &mut v.widgets;

    w.noninteractive.bg_fill = p.card;
    w.noninteractive.weak_bg_fill = p.card;
    w.noninteractive.bg_stroke = Stroke::new(1.0, p.border);
    w.noninteractive.fg_stroke = Stroke::new(1.0, p.text_muted);
    w.noninteractive.corner_radius = r;

    w.inactive.bg_fill = p.btn;
    w.inactive.weak_bg_fill = p.btn;
    w.inactive.bg_stroke = Stroke::new(1.0, p.border);
    w.inactive.fg_stroke = Stroke::new(1.0, p.text);
    w.inactive.corner_radius = r;

    w.hovered.bg_fill = p.btn_hover;
    w.hovered.weak_bg_fill = p.btn_hover;
    w.hovered.bg_stroke = Stroke::new(1.0, p.border_strong);
    w.hovered.fg_stroke = Stroke::new(1.0, p.text);
    w.hovered.corner_radius = r;

    w.active.bg_fill = p.btn_active;
    w.active.weak_bg_fill = p.btn_active;
    w.active.bg_stroke = Stroke::new(1.0, p.border_strong);
    w.active.fg_stroke = Stroke::new(1.0, p.text);
    w.active.corner_radius = r;

    w.open.bg_fill = p.btn;
    w.open.weak_bg_fill = p.btn;
    w.open.bg_stroke = Stroke::new(1.0, p.border_strong);
    w.open.fg_stroke = Stroke::new(1.0, p.text);
    w.open.corner_radius = r;

    style.visuals = v;
    // Pin the theme so egui stops following the OS theme; otherwise egui keeps
    // separate light/dark style slots and swaps to a default one at startup,
    // discarding the style we set here.
    let theme = if p.dark_mode { egui::Theme::Dark } else { egui::Theme::Light };
    ctx.set_theme(theme);
    ctx.set_global_style(style);
}

fn is_dark(ctx: &egui::Context) -> bool {
    ctx.global_style().visuals.dark_mode
}

pub fn sidebar_fill(ctx: &egui::Context) -> Color32 {
    if is_dark(ctx) { dark().sidebar } else { light().sidebar }
}

pub fn nav_selected_fill(ctx: &egui::Context) -> Color32 {
    if is_dark(ctx) { dark().nav_selected } else { light().nav_selected }
}

pub fn nav_hover_fill(ctx: &egui::Context) -> Color32 {
    if is_dark(ctx) { dark().btn_hover } else { light().btn_hover }
}

/// Muted secondary text for the current visuals.
pub fn text_muted(visuals: &egui::Visuals) -> Color32 {
    visuals.widgets.noninteractive.fg_stroke.color
}

/// Card / panel surface for the current visuals.
pub fn card_fill(visuals: &egui::Visuals) -> Color32 {
    visuals.extreme_bg_color
}

/// Hairline border for the current visuals.
pub fn border(visuals: &egui::Visuals) -> Color32 {
    visuals.widgets.noninteractive.bg_stroke.color
}
