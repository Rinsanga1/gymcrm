use eframe::egui::{self, Color32, CornerRadius, FontFamily, FontId, Stroke, TextStyle};

/// mdsfrontend brand accent (violet, brand-500). Focus / selection highlight only.
pub const ACCENT: Color32 = Color32::from_rgb(139, 92, 246);

/// Money/status semantics. Money is money in any theme, so these stay fixed
/// across light and dark rather than shifting with the palette. Values match
/// the mdsfrontend status tokens (success/error/warning).
pub const POSITIVE: Color32 = Color32::from_rgb(34, 197, 94);
pub const NEGATIVE: Color32 = Color32::from_rgb(239, 68, 68);
pub const WARNING: Color32 = Color32::from_rgb(245, 158, 11);

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
    accent: Color32,
}

fn light() -> Palette {
    // Light counterpart to the mdsfrontend dark design, built from its gray ramp
    // (gray-0..900): white canvas, gray-900 text, gray-200 borders, with
    // gray-100 -> 200 -> 300 button steps. Violet brand accent is shared.
    Palette {
        dark_mode: false,
        main: Color32::from_rgb(255, 255, 255),
        sidebar: Color32::from_rgb(249, 250, 251),
        card: Color32::from_rgb(255, 255, 255),
        faint: Color32::from_rgb(243, 244, 246),
        border: Color32::from_rgb(229, 231, 235),
        border_strong: Color32::from_rgb(209, 213, 219),
        text: Color32::from_rgb(11, 17, 32),
        text_muted: Color32::from_rgb(107, 114, 128),
        btn: Color32::from_rgb(243, 244, 246),
        btn_hover: Color32::from_rgb(229, 231, 235),
        btn_active: Color32::from_rgb(209, 213, 219),
        nav_selected: Color32::from_rgb(243, 244, 246),
        accent: ACCENT,
    }
}

fn dark() -> Palette {
    // mdsfrontend dark tokens: #0a0a0a canvas, #fafafa text, #404040 borders,
    // #171717 cards, with white-over-dark button steps (white/5 -> /10 -> /20
    // pre-blended over the #0a0a0a background).
    Palette {
        dark_mode: true,
        main: Color32::from_rgb(10, 10, 10),
        sidebar: Color32::from_rgb(10, 10, 10),
        card: Color32::from_rgb(23, 23, 23),
        faint: Color32::from_rgb(20, 20, 20),
        border: Color32::from_rgb(64, 64, 64),
        border_strong: Color32::from_rgb(115, 115, 115),
        text: Color32::from_rgb(250, 250, 250),
        text_muted: Color32::from_rgb(163, 163, 163),
        btn: Color32::from_rgb(22, 22, 22),
        btn_hover: Color32::from_rgb(35, 35, 35),
        btn_active: Color32::from_rgb(59, 59, 59),
        nav_selected: Color32::from_rgb(23, 23, 23),
        accent: ACCENT,
    }
}

/// Custom font family for headings: Geist SemiBold. egui's `strong()` only
/// brightens color, so a heavier face is the only way to get real weight in the
/// hierarchy (size + weight), matching the mdsfrontend type scale.
fn semibold() -> FontFamily {
    FontFamily::Name("semibold".into())
}

/// Install Geist (the mdsfrontend design-system font, SIL OFL, vendored under
/// assets/fonts) as the proportional face, plus Geist SemiBold as a `semibold`
/// family for headings. Bundled into the binary so it renders identically on
/// every machine with no system-font dependency. Runs once; the flag guards
/// against rebuilding the atlas on every theme re-apply.
fn install_fonts(ctx: &egui::Context) {
    if ctx.data(|d| d.get_temp::<bool>(egui::Id::new("fonts_installed"))) == Some(true) {
        return;
    }
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "geist".to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../../assets/fonts/Geist-Regular.ttf"
        ))),
    );
    fonts.font_data.insert(
        "geist_sb".to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../../assets/fonts/Geist-SemiBold.ttf"
        ))),
    );
    if let Some(list) = fonts.families.get_mut(&FontFamily::Proportional) {
        list.insert(0, "geist".to_owned());
    }
    let mut heavy = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    heavy.insert(0, "geist_sb".to_owned());
    fonts.families.insert(semibold(), heavy);
    ctx.set_fonts(fonts);
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("fonts_installed"), true));
}

/// Apply the theme to the whole context. Idempotent.
pub fn apply(ctx: &egui::Context, mode: Mode) {
    install_fonts(ctx);
    let p = match mode {
        Mode::Light => light(),
        Mode::Dark => dark(),
    };
    let mut style = (*ctx.global_style()).clone();

    // The mdsfrontend type scale, mapped onto egui's text styles: Heading = h4
    // (25), Body = body (16), Button = body-sm (14.4), Small = caption (12.8).
    // Body 16 also clears the "never below 16" readability floor for the
    // non-technical audience.
    style.text_styles = [
        (TextStyle::Heading, FontId::new(25.0, semibold())),
        (TextStyle::Body, FontId::new(16.0, FontFamily::Proportional)),
        (TextStyle::Button, FontId::new(14.4, FontFamily::Proportional)),
        (TextStyle::Small, FontId::new(12.8, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(14.0, FontFamily::Monospace)),
    ]
    .into();

    let s = &mut style.spacing;
    s.item_spacing = egui::vec2(10.0, 10.0);
    s.button_padding = egui::vec2(14.0, 8.0);
    s.menu_margin = egui::Margin::same(6);
    s.window_margin = egui::Margin::same(18);
    s.interact_size.y = 36.0;
    s.scroll.bar_width = 10.0;

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
    v.window_corner_radius = CornerRadius::same(0);
    v.menu_corner_radius = CornerRadius::same(0);
    v.hyperlink_color = p.accent;
    v.warn_fg_color = Color32::from_rgb(200, 130, 50);
    v.error_fg_color = Color32::from_rgb(210, 90, 90);

    v.selection.bg_fill = p.accent.gamma_multiply(if p.dark_mode { 0.45 } else { 0.22 });
    v.selection.stroke = Stroke::new(1.0, p.accent);

    // mdsfrontend uses sharp, effectively-square corners.
    let r = CornerRadius::same(0);
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
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("theme_mode"), mode));
}

fn current_palette(ctx: &egui::Context) -> Palette {
    let mode = ctx
        .data(|d| d.get_temp::<Mode>(egui::Id::new("theme_mode")))
        .unwrap_or(Mode::Dark);
    match mode {
        Mode::Light => light(),
        Mode::Dark => dark(),
    }
}

pub fn sidebar_fill(ctx: &egui::Context) -> Color32 {
    current_palette(ctx).sidebar
}

pub fn nav_selected_fill(ctx: &egui::Context) -> Color32 {
    current_palette(ctx).nav_selected
}

pub fn nav_hover_fill(ctx: &egui::Context) -> Color32 {
    current_palette(ctx).btn_hover
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
