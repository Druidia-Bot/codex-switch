//! codex-switch-bar — a tiny floating toolbar for flipping Codex between
//! OpenAI and OpenRouter.
//!
//! Toggle OFF → your original OpenAI config, restored exactly. Toggle ON →
//! OpenRouter with the model you last used; the dropdown lists every OpenRouter
//! model (recent and popular first). Gear → store your OpenRouter API key.
//! Flips are hot: the next Codex thread (app or CLI) uses the new provider.

#![windows_subsystem = "windows"]

use codex_switch::*;
use eframe::egui::{self, Align, Color32, FontId, Layout, RichText, Stroke};
use std::sync::mpsc::{channel, Receiver};
use std::time::{Duration, Instant};

const WIDTH: f32 = 360.0;

// Spacing scale: 4 · 8 · 12 · 16 · 24
const SP_SM: f32 = 8.0;
const SP_MD: f32 = 12.0;
const SP_LG: f32 = 16.0;

// Type scale: 11 · 12 · 13 · 15
const FS_MICRO: f32 = 11.0;
const FS_SMALL: f32 = 12.0;
const FS_BODY: f32 = 13.0;
const FS_TITLE: f32 = 15.0;

const RADIUS: f32 = 5.0;
const ROW_H: f32 = 28.0;

const BG: Color32 = Color32::from_rgb(24, 25, 29);
const BG_EDGE: Color32 = Color32::from_rgb(52, 54, 62);
const BG_FIELD: Color32 = Color32::from_rgb(17, 18, 21);
const BG_RAISED: Color32 = Color32::from_rgb(36, 38, 44);
const TEXT: Color32 = Color32::from_rgb(232, 232, 237);
const TEXT_SEC: Color32 = Color32::from_rgb(150, 153, 163);
const TEXT_TER: Color32 = Color32::from_rgb(105, 108, 118);
const ACCENT: Color32 = Color32::from_rgb(120, 140, 255); // openrouter indigo
const OPENAI_GREEN: Color32 = Color32::from_rgb(110, 200, 130);
const ERR_RED: Color32 = Color32::from_rgb(235, 110, 110);
const WARN: Color32 = Color32::from_rgb(230, 190, 90);
const TRACK_OFF: Color32 = Color32::from_rgb(58, 60, 68);

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([WIDTH, 124.0])
            .with_decorations(false)
            .with_transparent(true)
            .with_resizable(false),
        ..Default::default()
    };
    eframe::run_native(
        "codex-switch",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}

enum View {
    Main,
    Picker,
    Settings,
}

/// Result of the background catalog job.
struct CatalogReady {
    cache: OrCache,
    err: Option<String>,
}

struct App {
    paths: Paths,
    view: View,
    on: bool,
    /// Model currently written in the config (whichever side is active).
    active_model: String,
    /// Last-known OpenRouter selection (shown on the dropdown button).
    or_model: Option<String>,
    cache: Option<OrCache>,
    catalog_rx: Option<Receiver<CatalogReady>>,
    fetch_err: Option<String>,
    search: String,
    api_key_input: String,
    key_present: bool,
    key_label: Option<&'static str>,
    key_rx: Option<Receiver<Result<String, String>>>,
    err: Option<String>,
    info: Option<String>,
    restart_hint: bool,
    last_poll: Instant,
    last_height: f32,
    /// Keep the bar above other windows (off by default; pin to enable).
    pinned: bool,
    /// Whether the Segoe Fluent/MDL2 icon font was loaded from Windows.
    icon_font: bool,
}

/// Segoe Fluent Icons / MDL2 Assets glyphs — the same ones Windows itself
/// uses for caption buttons.
mod glyph {
    pub const MINIMIZE: &str = "\u{E921}";
    pub const CLOSE: &str = "\u{E8BB}";
    pub const SETTINGS: &str = "\u{E713}";
    pub const PIN: &str = "\u{E718}";
    pub const UNPIN: &str = "\u{E77A}";
    pub const REFRESH: &str = "\u{E72C}";
}

const ICON_FAMILY: &str = "windows-icons";

/// Load the system caption-icon font so our title bar matches Windows.
fn load_windows_icon_font(ctx: &egui::Context) -> bool {
    for path in [
        "C:\\Windows\\Fonts\\SegoeIcons.ttf", // Segoe Fluent Icons (Win 11)
        "C:\\Windows\\Fonts\\segmdl2.ttf",    // Segoe MDL2 Assets (Win 10)
    ] {
        if let Ok(bytes) = std::fs::read(path) {
            let mut fonts = egui::FontDefinitions::default();
            fonts.font_data.insert(
                "segoe_icons".into(),
                egui::FontData::from_owned(bytes).into(),
            );
            fonts
                .families
                .entry(egui::FontFamily::Name(ICON_FAMILY.into()))
                .or_default()
                .push("segoe_icons".into());
            ctx.set_fonts(fonts);
            return true;
        }
    }
    false
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut visuals = egui::Visuals::dark();
        visuals.extreme_bg_color = BG_FIELD; // text-edit backgrounds
        visuals.selection.bg_fill = ACCENT.gamma_multiply(0.25);
        visuals.selection.stroke = Stroke::new(1.0, ACCENT);
        visuals.widgets.hovered.bg_fill = BG_RAISED;
        visuals.widgets.active.bg_fill = BG_RAISED;
        for w in [
            &mut visuals.widgets.noninteractive,
            &mut visuals.widgets.inactive,
            &mut visuals.widgets.hovered,
            &mut visuals.widgets.active,
            &mut visuals.widgets.open,
        ] {
            w.corner_radius = RADIUS.into();
        }
        let icon_font = load_windows_icon_font(&cc.egui_ctx);
        cc.egui_ctx.set_visuals(visuals);
        cc.egui_ctx.style_mut(|style| {
            style.spacing.item_spacing = egui::vec2(10.0, SP_SM);
            style.spacing.button_padding = egui::vec2(10.0, 5.0);
        });

        let paths = Paths::resolve(None).expect("could not resolve ~/.codex");
        let state = State::load(&paths.state);
        let key_label = key_source_label(&paths);

        let mut app = App {
            view: View::Main,
            on: false,
            active_model: "(unset)".into(),
            or_model: state.last_openrouter_model.clone(),
            cache: None,
            catalog_rx: None,
            fetch_err: None,
            search: String::new(),
            api_key_input: String::new(),
            key_present: key_label.is_some(),
            key_label,
            key_rx: None,
            err: None,
            info: None,
            restart_hint: false,
            last_poll: Instant::now(),
            last_height: 0.0,
            pinned: false,
            icon_font,
            paths,
        };
        app.refresh_status();
        if app.on {
            app.or_model = Some(app.active_model.clone());
        }
        app.start_catalog(false);
        app
    }

    /// Fetch the OpenRouter list, refresh the native cache, regenerate the
    /// combined catalog, and register it — all off the UI thread.
    fn start_catalog(&mut self, force: bool) {
        if self.catalog_rx.is_some() {
            return;
        }
        let (tx, rx) = channel();
        self.catalog_rx = Some(rx);
        self.fetch_err = None;
        let paths = Paths::resolve(None).expect("could not resolve ~/.codex");
        std::thread::spawn(move || {
            let result = rebuild_catalog(&paths, force)
                .and_then(|(cache, _)| ensure_installed(&paths).map(|_| cache));
            let msg = match result {
                Ok(cache) => CatalogReady { cache, err: None },
                Err(e) => CatalogReady {
                    // Keep whatever cached list we have so the picker still works offline.
                    cache: load_openrouter(&paths, false).unwrap_or_default(),
                    err: Some(format!("{e:#}")),
                },
            };
            let _ = tx.send(msg);
        });
    }

    fn poll_background(&mut self) {
        if let Some(rx) = &self.catalog_rx {
            if let Ok(ready) = rx.try_recv() {
                if !ready.cache.models.is_empty() {
                    self.cache = Some(ready.cache);
                }
                self.fetch_err = ready.err;
                self.catalog_rx = None;
                self.restart_hint = desktop_needs_restart(&self.paths);
            }
        }
        if let Some(rx) = &self.key_rx {
            if let Ok(res) = rx.try_recv() {
                match res {
                    Ok(info) => {
                        self.key_present = true;
                        self.key_label = key_source_label(&self.paths);
                        self.api_key_input.clear();
                        self.err = None;
                        self.info = Some(format!("Key saved · {info}"));
                        self.view = View::Main;
                    }
                    Err(e) => self.err = Some(e),
                }
                self.key_rx = None;
            }
        }
        // Reflect changes made by the CLI or by hand while the bar is open.
        if self.last_poll.elapsed() > Duration::from_millis(1500) {
            self.last_poll = Instant::now();
            self.refresh_status();
        }
    }

    fn refresh_status(&mut self) {
        if let Ok(doc) = load_doc(&self.paths.config) {
            let (provider, model) = current(&doc);
            self.on = provider == PROVIDER_ID;
            self.active_model = model;
            if self.on {
                self.or_model = Some(self.active_model.clone());
            }
        }
    }

    fn go_openai(&mut self) {
        self.err = None;
        if let Err(e) = switch_off(&self.paths) {
            self.err = Some(format!("{e:#}"));
        }
        self.refresh_status();
    }

    fn go_openrouter(&mut self, model: &str) {
        self.err = None;
        let Some(key) = key_source(&self.paths) else {
            self.err = Some("No OpenRouter key — click ⚙ to add one".into());
            return;
        };
        if let Err(e) = ensure_installed(&self.paths) {
            self.err = Some(format!("{e:#}"));
            return;
        }
        match switch_on(&self.paths, model, &key) {
            Ok(()) => {
                self.or_model = Some(model.to_string());
                self.restart_hint = desktop_needs_restart(&self.paths);
            }
            Err(e) => self.err = Some(format!("{e:#}")),
        }
        self.refresh_status();
    }

    fn save_api_key(&mut self) {
        let key = self.api_key_input.trim().to_string();
        if key.is_empty() {
            self.err = Some("key is empty".into());
            return;
        }
        if self.key_rx.is_some() {
            return;
        }
        let (tx, rx) = channel();
        self.key_rx = Some(rx);
        self.info = Some("Verifying key with OpenRouter…".into());
        let paths = Paths::resolve(None).expect("could not resolve ~/.codex");
        std::thread::spawn(move || {
            let _ = tx.send(save_key(&paths, &key).map_err(|e| format!("{e:#}")));
        });
    }

    // ------------------------------------------------------------- ui

    fn key_warning_visible(&self) -> bool {
        self.on && !self.key_present && self.err.is_none() && self.info.is_none()
    }

    fn desired_height(&self) -> f32 {
        // 26 card margins + 26 header + per-view content. Gaps count both the
        // explicit add_space and egui's 8px automatic item spacing.
        match self.view {
            View::Main => {
                let mut h = 52.0 + 20.0 + 28.0 + 24.0 + 18.0 + 6.0 + 46.0; // header gap, toggle, gap, status, hint
                if self.on {
                    h += 20.0 + 34.0; // gap + dropdown button
                }
                if self.key_warning_visible() || self.restart_hint {
                    h += 26.0;
                }
                h
            }
            View::Picker => 496.0,
            View::Settings => {
                let mut h = 52.0 + 20.0 + 18.0 + 16.0 + 32.0 + 20.0 + 30.0 + 12.0 + 50.0;
                if self.err.is_some() || self.info.is_some() {
                    h += 26.0;
                }
                h
            }
        }
    }

    fn header(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        // Drag zone registered first; buttons drawn after still win hit-tests.
        let bar_rect = {
            let mut r = ui.max_rect();
            r.max.y = r.min.y + 26.0;
            r
        };
        let drag = ui.interact(
            bar_rect,
            egui::Id::new("titlebar-drag"),
            egui::Sense::click_and_drag(),
        );
        if drag.drag_started_by(egui::PointerButton::Primary) {
            ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }

        let icon_font = self.icon_font;
        ui.scope_builder(egui::UiBuilder::new().max_rect(bar_rect), |ui| {
            ui.horizontal_centered(|ui| {
                ui.label(RichText::new("codex-switch").size(FS_SMALL).color(TEXT_TER));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    if caption_button(ui, glyph::CLOSE, "×", icon_font, true, false).clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    if caption_button(ui, glyph::MINIMIZE, "–", icon_font, false, false)
                        .clicked()
                    {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                    }
                    let pin_glyph = if self.pinned { glyph::UNPIN } else { glyph::PIN };
                    if caption_button(ui, pin_glyph, "^", icon_font, false, self.pinned)
                        .on_hover_text(if self.pinned {
                            "Unpin (stop floating on top)"
                        } else {
                            "Pin on top of other windows"
                        })
                        .clicked()
                    {
                        self.pinned = !self.pinned;
                        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
                            if self.pinned {
                                egui::WindowLevel::AlwaysOnTop
                            } else {
                                egui::WindowLevel::Normal
                            },
                        ));
                    }
                    if caption_button(ui, glyph::SETTINGS, "⚙", icon_font, false, false)
                        .on_hover_text("OpenRouter API key")
                        .clicked()
                    {
                        self.info = None;
                        self.err = None;
                        self.view = match self.view {
                            View::Settings => View::Main,
                            _ => View::Settings,
                        };
                    }
                    let busy = self.catalog_rx.is_some();
                    if caption_button(ui, glyph::REFRESH, "↻", icon_font, false, busy)
                        .on_hover_text("Re-fetch the OpenRouter catalog")
                        .clicked()
                        && !busy
                    {
                        self.start_catalog(true);
                    }
                });
            });
        });
    }

    fn main_view(&mut self, ui: &mut egui::Ui) {
        ui.add_space(SP_MD);

        // Hero row: provider toggle. Emphasis carried by colour, not size,
        // so nothing reflows when it flips.
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = SP_MD;
            ui.label(
                RichText::new("OpenAI")
                    .size(FS_TITLE)
                    .strong()
                    .color(if self.on { TEXT_TER } else { OPENAI_GREEN }),
            );
            let mut on = self.on;
            if toggle_switch(ui, &mut on).changed() {
                self.info = None;
                if on {
                    match self.or_model.clone() {
                        Some(m) => self.go_openrouter(&m),
                        None => {
                            // No model chosen yet — open the picker first.
                            self.view = View::Picker;
                        }
                    }
                } else {
                    self.go_openai();
                }
            }
            ui.label(
                RichText::new("OpenRouter")
                    .size(FS_TITLE)
                    .strong()
                    .color(if self.on { ACCENT } else { TEXT_TER }),
            );
        });

        if self.on {
            ui.add_space(SP_MD);
            let text = self
                .or_model
                .clone()
                .unwrap_or_else(|| "Choose a model…".into());
            if dropdown_button(ui, &text).on_hover_text("Choose an OpenRouter model").clicked() {
                self.search.clear();
                self.view = View::Picker;
            }
        }

        ui.add_space(SP_LG);
        self.status_line(ui);
        ui.add_space(6.0);
        let hint = if self.on {
            "The Codex app's model picker shows this same list. Pick a model here (used by the CLI and as the default) or in the app; new threads go through OpenRouter."
        } else {
            "The Codex app's picker still lists the “· OpenRouter” models. Flip ON before choosing one there, or the request goes to OpenAI and fails."
        };
        ui.add(egui::Label::new(RichText::new(hint).size(FS_MICRO).color(TEXT_TER)).wrap());
    }

    fn status_line(&self, ui: &mut egui::Ui) {
        if let Some(e) = &self.err {
            ui.label(RichText::new(e).size(FS_SMALL).color(ERR_RED));
            return;
        }
        if let Some(i) = &self.info {
            ui.label(RichText::new(i).size(FS_SMALL).color(TEXT_SEC));
            return;
        }
        let (dot, name) = if self.on {
            (ACCENT, "OpenRouter")
        } else {
            (OPENAI_GREEN, "OpenAI")
        };
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = SP_SM;
            let (dot_rect, _) =
                ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
            ui.painter().circle_filled(dot_rect.center(), 3.0, dot);
            let mut line = format!("{name} · {}", self.active_model);
            if self.catalog_rx.is_some() {
                line.push_str("  · syncing catalog…");
            }
            ui.label(RichText::new(line).size(FS_SMALL).color(TEXT_SEC));
        });
        if self.key_warning_visible() {
            ui.label(
                RichText::new("No API key set — click ⚙")
                    .size(FS_SMALL)
                    .color(ERR_RED),
            );
        } else if self.restart_hint {
            ui.label(
                RichText::new("Restart the Codex app once so its picker lists OpenRouter models")
                    .size(FS_SMALL)
                    .color(WARN),
            )
            .on_hover_text("The app caches its model list at startup. Provider flips already apply to its next thread.");
        }
    }

    fn picker_view(&mut self, ui: &mut egui::Ui) {
        ui.add_space(SP_MD);
        ui.horizontal(|ui| {
            let search = egui::TextEdit::singleline(&mut self.search)
                .font(FontId::proportional(FS_BODY))
                .hint_text(RichText::new("Search models…").size(FS_BODY).color(TEXT_TER))
                .margin(egui::vec2(10.0, 6.0))
                .desired_width(ui.available_width() - 56.0);
            let resp = ui.add(search);
            resp.request_focus();
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if flat_button(ui, "Back").clicked() {
                    self.view = View::Main;
                }
            });
        });
        ui.add_space(4.0);

        let Some(cache) = &self.cache else {
            if self.catalog_rx.is_some() {
                ui.label(RichText::new("Loading catalog…").size(FS_SMALL).color(TEXT_SEC));
            } else if let Some(e) = self.fetch_err.clone() {
                ui.label(RichText::new(e).size(FS_SMALL).color(ERR_RED));
                ui.add_space(SP_SM);
                if ui.button(RichText::new("Retry").size(FS_SMALL)).clicked() {
                    self.start_catalog(true);
                }
            }
            return;
        };

        // Flatten sections into rows; a header row precedes each section.
        enum Row {
            Header(&'static str),
            Model(ModelEntry),
        }
        let state = State::load(&self.paths.state);
        let needle = self.search.trim().to_lowercase();
        let mut rows: Vec<Row> = Vec::new();
        let mut shown = 0usize;
        for (section, entries) in sectioned(cache, &state) {
            let filtered: Vec<ModelEntry> = entries
                .into_iter()
                .filter(|m| {
                    needle.is_empty()
                        || m.id.to_lowercase().contains(&needle)
                        || m.name.as_deref().map(|n| n.to_lowercase().contains(&needle)).unwrap_or(false)
                })
                .collect();
            if filtered.is_empty() {
                continue;
            }
            rows.push(Row::Header(section));
            shown += filtered.len();
            rows.extend(filtered.into_iter().map(Row::Model));
        }

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            ui.label(
                RichText::new(format!("{} of {} models", shown, cache.models.len()))
                    .size(FS_MICRO)
                    .color(TEXT_TER),
            );
            if let Some(e) = &self.fetch_err {
                ui.label(RichText::new("· offline list").size(FS_MICRO).color(WARN))
                    .on_hover_text(e);
            }
        });
        ui.add_space(2.0);

        let mut chosen: Option<String> = None;
        let row_height = ROW_H;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show_rows(ui, row_height, rows.len(), |ui, range| {
                ui.spacing_mut().item_spacing.y = 0.0;
                for i in range {
                    match &rows[i] {
                        Row::Header(h) => section_header(ui, h),
                        Row::Model(m) => {
                            let selected = self.or_model.as_deref() == Some(m.id.as_str());
                            if model_row(ui, &m.id, m.meta(), selected, !m.supports_tools()).clicked() {
                                chosen = Some(m.id.clone());
                            }
                        }
                    }
                }
            });

        if let Some(m) = chosen {
            self.go_openrouter(&m);
            self.view = View::Main;
        }
        ui.add_space(4.0);
        ui.add(egui::Label::new(
            RichText::new("Same list as the Codex app's picker; the app re-reads it when it restarts.")
                .size(FS_MICRO)
                .color(TEXT_TER),
        ).wrap());
    }

    fn settings_view(&mut self, ui: &mut egui::Ui) {
        ui.add_space(SP_MD);
        ui.label(
            RichText::new("OpenRouter API key")
                .size(FS_BODY)
                .strong()
                .color(TEXT),
        );
        ui.add_space(SP_SM);
        let hint = match self.key_label {
            Some(src) => format!("Key in use from {src} — enter to replace"),
            None => "sk-or-v1-…".to_string(),
        };
        let field = egui::TextEdit::singleline(&mut self.api_key_input)
            .password(true)
            .font(FontId::proportional(FS_BODY))
            .hint_text(RichText::new(hint).size(FS_BODY).color(TEXT_TER))
            .margin(egui::vec2(10.0, 7.0))
            .desired_width(ui.available_width());
        let resp = ui.add(field);
        ui.add_space(SP_MD);
        ui.horizontal(|ui| {
            let save = ui.add(
                egui::Button::new(
                    RichText::new("Save").size(FS_BODY).strong().color(Color32::BLACK),
                )
                .fill(ACCENT)
                .corner_radius(RADIUS)
                .min_size(egui::vec2(76.0, 30.0)),
            );
            if save.clicked() || (resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
            {
                self.save_api_key();
            }
            if ui
                .add(
                    egui::Button::new(RichText::new("Cancel").size(FS_SMALL).color(TEXT_SEC))
                        .fill(Color32::TRANSPARENT)
                        .stroke(Stroke::NONE)
                        .min_size(egui::vec2(0.0, 30.0)),
                )
                .clicked()
            {
                self.api_key_input.clear();
                self.err = None;
                self.info = None;
                self.view = View::Main;
            }
        });
        ui.add_space(4.0);
        ui.label(
            RichText::new("Verified against OpenRouter, then stored encrypted (Windows DPAPI). While ON it is written into config.toml so the running Codex app can use it.")
                .size(FS_MICRO)
                .color(TEXT_TER),
        );
        if let Some(e) = &self.err {
            ui.add_space(4.0);
            ui.label(RichText::new(e).size(FS_SMALL).color(ERR_RED));
        } else if let Some(i) = &self.info {
            ui.add_space(4.0);
            ui.label(RichText::new(i).size(FS_SMALL).color(TEXT_SEC));
        }
    }
}

impl eframe::App for App {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0] // fully transparent — we draw our own rounded card
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_background();
        if self.catalog_rx.is_some() || self.key_rx.is_some() {
            ctx.request_repaint_after(Duration::from_millis(150));
        } else {
            ctx.request_repaint_after(Duration::from_millis(1500));
        }

        let h = self.desired_height();
        if (h - self.last_height).abs() > 0.5 {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(WIDTH, h)));
            self.last_height = h;
        }

        let card = egui::Frame::new()
            .fill(BG)
            .stroke(Stroke::new(1.0, BG_EDGE))
            .corner_radius(8.0)
            .inner_margin(egui::Margin {
                left: 16,
                right: 16,
                top: 12,
                bottom: 14,
            });

        egui::CentralPanel::default().frame(card).show(ctx, |ui| {
            self.header(ui, ctx);
            match self.view {
                View::Main => self.main_view(ui),
                View::Picker => self.picker_view(ui),
                View::Settings => self.settings_view(ui),
            }
        });
    }
}

// ---------------------------------------------------------------- widgets

/// Single-line galley truncated with an ellipsis at `max_width`.
fn truncated(ui: &egui::Ui, text: &str, font: FontId, color: Color32, max_width: f32) -> std::sync::Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::simple_singleline(text.to_owned(), font, color);
    job.wrap = egui::text::TextWrapping {
        max_width: max_width.max(0.0),
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('\u{2026}'),
    };
    ui.fonts(|f| f.layout_job(job))
}

/// Left-aligned list row: model id on the left, metadata right-aligned in a
/// quieter type, both truncated rather than wrapped. Selection is a flat
/// tint with an accent bar; hover is a subtle fill.
fn model_row(ui: &mut egui::Ui, id: &str, meta: Option<String>, selected: bool, dim: bool) -> egui::Response {
    let pad = 10.0;
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, ROW_H), egui::Sense::click());
    if ui.is_rect_visible(rect) {
        if selected {
            ui.painter().rect_filled(rect, 4.0, ACCENT.gamma_multiply(0.16));
            ui.painter().rect_filled(
                egui::Rect::from_min_size(rect.min, egui::vec2(2.0, rect.height())),
                1.0,
                ACCENT,
            );
        } else if resp.hovered() {
            ui.painter().rect_filled(rect, 4.0, BG_RAISED);
        }
        let meta_galley = meta.map(|m| truncated(ui, &m, FontId::proportional(FS_MICRO), TEXT_TER, w * 0.5));
        let meta_w = meta_galley.as_ref().map(|g| g.size().x + pad).unwrap_or(0.0);
        let id_color = if selected {
            ACCENT
        } else if dim {
            TEXT_SEC
        } else {
            TEXT
        };
        let id_galley = truncated(ui, id, FontId::proportional(FS_BODY), id_color, w - pad * 2.0 - meta_w);
        let y = rect.center().y - id_galley.size().y / 2.0;
        ui.painter().galley(egui::pos2(rect.min.x + pad, y), id_galley, id_color);
        if let Some(g) = meta_galley {
            let y = rect.center().y - g.size().y / 2.0;
            ui.painter().galley(egui::pos2(rect.max.x - pad - g.size().x, y), g, TEXT_TER);
        }
    }
    resp
}

/// Quiet uppercase section label with a hairline underneath.
fn section_header(ui: &mut egui::Ui, text: &str) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), ROW_H), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        ui.painter().text(
            egui::pos2(rect.min.x + 10.0, rect.max.y - 9.0),
            egui::Align2::LEFT_BOTTOM,
            text.to_uppercase(),
            FontId::proportional(FS_MICRO - 1.0),
            TEXT_TER,
        );
        ui.painter().hline(
            (rect.min.x + 10.0)..=(rect.max.x - 10.0),
            rect.max.y - 4.0,
            Stroke::new(1.0, BG_EDGE),
        );
    }
}

/// Full-width select-style button: left-aligned label, chevron on the right.
fn dropdown_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 32.0), egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let hovered = resp.hovered();
        ui.painter().rect(
            rect,
            RADIUS,
            if hovered { BG_RAISED.gamma_multiply(1.25) } else { BG_RAISED },
            Stroke::new(1.0, if hovered { TEXT_TER } else { BG_EDGE }),
            egui::StrokeKind::Inside,
        );
        let g = truncated(ui, text, FontId::proportional(FS_BODY), TEXT, rect.width() - 44.0);
        ui.painter().galley(egui::pos2(rect.min.x + 10.0, rect.center().y - g.size().y / 2.0), g, TEXT);
        let c = egui::pos2(rect.max.x - 16.0, rect.center().y - 1.0);
        let s = Stroke::new(1.5, TEXT_SEC);
        ui.painter().line_segment([egui::pos2(c.x - 4.0, c.y - 2.0), egui::pos2(c.x, c.y + 2.0)], s);
        ui.painter().line_segment([egui::pos2(c.x, c.y + 2.0), egui::pos2(c.x + 4.0, c.y - 2.0)], s);
    }
    resp
}

/// Text-only secondary action.
fn flat_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(text).size(FS_SMALL).color(TEXT_SEC))
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke::NONE)
            .min_size(egui::vec2(0.0, 30.0)),
    )
}

/// A Windows-style caption button: quiet until hovered, red hover for close,
/// using the system Segoe icon font when available.
fn caption_button(
    ui: &mut egui::Ui,
    icon: &str,
    fallback: &str,
    icon_font: bool,
    is_close: bool,
    active: bool,
) -> egui::Response {
    let size = egui::vec2(30.0, 24.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let hovered = response.hovered();
        if hovered {
            let bg = if is_close {
                Color32::from_rgb(196, 43, 28) // Windows close-button red
            } else {
                BG_RAISED
            };
            ui.painter().rect_filled(rect, 6.0, bg);
        }
        let color = if hovered && is_close {
            Color32::WHITE
        } else if hovered {
            TEXT
        } else if active {
            ACCENT
        } else {
            TEXT_SEC
        };
        let (label, font) = if icon_font {
            (
                icon,
                FontId::new(10.0, egui::FontFamily::Name(ICON_FAMILY.into())),
            )
        } else {
            (fallback, FontId::proportional(14.0))
        };
        ui.painter()
            .text(rect.center(), egui::Align2::CENTER_CENTER, label, font, color);
    }
    response
}

/// iOS-style animated toggle switch.
fn toggle_switch(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let size = egui::vec2(46.0, 26.0);
    let (rect, mut response) = ui.allocate_exact_size(size, egui::Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    if ui.is_rect_visible(rect) {
        let t = ui.ctx().animate_bool(response.id, *on);
        let mut track = lerp_color(TRACK_OFF, ACCENT, t);
        if response.hovered() {
            track = track.gamma_multiply(1.15);
        }
        let radius = rect.height() / 2.0;
        ui.painter().rect_filled(rect, radius, track);
        let cx = egui::lerp((rect.left() + radius)..=(rect.right() - radius), t);
        ui.painter().circle_filled(
            egui::pos2(cx, rect.center().y),
            radius - 3.0,
            Color32::WHITE,
        );
    }
    response
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t) as u8;
    Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}
