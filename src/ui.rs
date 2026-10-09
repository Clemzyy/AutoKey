//! Interface (egui) : clavier AZERTY, programmation, liste d'actions, options.
use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use eframe::egui::{
    self, pos2, vec2, Align, Align2, Color32, CornerRadius, CursorIcon, FontData, FontDefinitions, FontFamily,
    FontId, Frame, Id, Layout, Margin, Rect, Response, RichText, ScrollArea, Sense, Stroke, Ui, ViewportCommand,
};
use egui::text::{CCursor, CCursorRange};
use egui::text_edit::TextEditState;

use crate::engine::{self, Target};
use crate::i18n::{self, tf, Lang, Msg};
use crate::keys::{self, KbLayout, KeyDef, Modk, Tok, LAYOUTS};
use crate::model::{self, Action, Saved};
use crate::worker::{self, Kind, Shared};

/// Mode de test (AUTOKEY_STATE) : chaque composant interactif enregistre sa position pour que le script de test puisse le cliquer.
static DEV: AtomicBool = AtomicBool::new(false);
thread_local! {
    static WIDGETS: RefCell<Vec<(String, Rect)>> = const { RefCell::new(Vec::new()) };
}

fn reg(name: impl Into<String>, rect: Rect) {
    if DEV.load(Ordering::Relaxed) {
        WIDGETS.with(|w| w.borrow_mut().push((name.into(), rect)));
    }
}

const AUTHOR: &str = "Clemzy aka InforMagicien";
const DONATE_URL: &str = "https://www.paypal.com/donate/?hosted_button_id=NKCR6KK739WGS";

// ---------- thème ----------
const fn hex(c: u32) -> Color32 {
    Color32::from_rgb((c >> 16) as u8, (c >> 8) as u8, c as u8)
}
const BG: Color32 = hex(0x0d0f17);
const CARD: Color32 = hex(0x161a26);
const FIELD: Color32 = hex(0x1e2333);
const BORDER: Color32 = hex(0x2a3047);
const KEY: Color32 = hex(0x242a3d);
const KEY_HOV: Color32 = hex(0x343d5c);
const TXT: Color32 = hex(0xe9ecf8);
const MUTED: Color32 = hex(0x8a91ad);
const ACCENT: Color32 = hex(0x7c5cff);
const ACCENT_HOV: Color32 = hex(0x9378ff);
const GREEN: Color32 = hex(0x22c55e);
const GREEN_HOV: Color32 = hex(0x34d474);
const RED: Color32 = hex(0xef4444);
const RED_HOV: Color32 = hex(0xf66464);
const VIOLET: Color32 = hex(0xc9b1ff);
const JAUNE: Color32 = hex(0xf5d63d);
const ORANGE: Color32 = hex(0xf08a24);
const GRIS: Color32 = hex(0x3a4058);
const ROW: Color32 = hex(0x1c2132);
const ROW_SEL: Color32 = hex(0x2f2a5c);
const PAYPAL: Color32 = hex(0x0070ba);
const PAYPAL_HOV: Color32 = hex(0x1a8ad6);
const DARK: Color32 = hex(0x111111);

fn bold_family() -> FontFamily {
    FontFamily::Name("bold".into())
}
fn fid(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}
fn fid_b(size: f32) -> FontId {
    FontId::new(size, bold_family())
}
fn fid_m(size: f32) -> FontId {
    FontId::new(size, FontFamily::Monospace)
}

/// Chemin (et indice dans la collection) d'une police : fichier de Windows, chemin absolu, ou motif fontconfig « fc:… ».
#[cfg(windows)]
fn resolve(file: &str) -> Option<(std::path::PathBuf, u32)> {
    Some((std::path::Path::new(r"C:\Windows\Fonts").join(file), 0))
}

#[cfg(not(windows))]
fn resolve(file: &str) -> Option<(std::path::PathBuf, u32)> {
    let Some(pattern) = file.strip_prefix("fc:") else {
        return Some((std::path::PathBuf::from(file), 0));
    };
    // demande à fontconfig la police qui correspond (« sans-serif:bold », « :lang=zh-cn »…)
    let out = std::process::Command::new("fc-match").args(["-f", "%{file}\n%{index}", pattern]).output().ok()?;
    let text = String::from_utf8(out.stdout).ok()?;
    let mut lines = text.lines();
    let path = lines.next()?.trim();
    (!path.is_empty()).then(|| (std::path::PathBuf::from(path), lines.next().and_then(|i| i.trim().parse().ok()).unwrap_or(0)))
}

#[cfg(windows)]
mod sysfonts {
    pub const REGULAR: &[&str] = &["segoeui.ttf", "arial.ttf"];
    pub const BOLD: &[&str] = &["segoeuib.ttf", "arialbd.ttf"];
    pub const SYMBOLS: &[&str] = &["seguisym.ttf"];
    pub const MONO: &[&str] = &["consola.ttf", "cour.ttf"];
    pub const HAN: &[&str] = &["msyh.ttc", "simsun.ttc"];
    pub const HAN_BOLD: &[&str] = &["msyhbd.ttc"];
}

#[cfg(target_os = "linux")]
mod sysfonts {
    pub const REGULAR: &[&str] = &["/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", "/usr/share/fonts/TTF/DejaVuSans.ttf", "fc:sans-serif"];
    pub const BOLD: &[&str] = &["/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf", "/usr/share/fonts/TTF/DejaVuSans-Bold.ttf", "fc:sans-serif:bold"];
    pub const SYMBOLS: &[&str] = &["/usr/share/fonts/truetype/noto/NotoSansSymbols2-Regular.ttf", "fc:symbols"];
    pub const MONO: &[&str] = &["/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf", "/usr/share/fonts/TTF/DejaVuSansMono.ttf", "fc:monospace"];
    pub const HAN: &[&str] = &["fc::lang=zh-cn"];
    pub const HAN_BOLD: &[&str] = &["fc::lang=zh-cn:bold"];
}

#[cfg(target_os = "macos")]
mod sysfonts {
    // Arial Unicode couvre le latin, le cyrillique et l'arabe ; Apple Symbols complète les pictogrammes.
    pub const REGULAR: &[&str] = &["/System/Library/Fonts/Supplemental/Arial Unicode.ttf", "/Library/Fonts/Arial Unicode.ttf", "/System/Library/Fonts/Helvetica.ttc"];
    pub const BOLD: &[&str] = &["/System/Library/Fonts/Supplemental/Arial Bold.ttf", "/System/Library/Fonts/Helvetica.ttc"];
    pub const SYMBOLS: &[&str] = &["/System/Library/Fonts/Apple Symbols.ttf"];
    pub const MONO: &[&str] = &["/System/Library/Fonts/Menlo.ttc", "/System/Library/Fonts/Supplemental/Courier New.ttf"];
    pub const HAN: &[&str] = &["/System/Library/Fonts/PingFang.ttc", "/System/Library/Fonts/Hiragino Sans GB.ttc", "/System/Library/Fonts/STHeiti Medium.ttc"];
    pub const HAN_BOLD: &[&str] = &["/System/Library/Fonts/PingFang.ttc"];
}

/// Polices système : egui n'embarque aucune police (exécutable plus léger de ~1,4 Mo).
/// La police principale couvre le latin, le cyrillique et l'arabe ; la police chinoise n'est chargée que si le chinois est utilisé.
fn setup_fonts(ctx: &egui::Context, cjk: bool) {
    let mut fonts = FontDefinitions::default();
    let mut add = |key: &str, files: &[&str]| -> bool {
        for (path, index) in files.iter().filter_map(|f| resolve(f)) {
            if let Ok(bytes) = std::fs::read(&path) {
                let mut data = FontData::from_owned(bytes);
                data.index = index;
                fonts.font_data.insert(key.to_string(), Arc::new(data));
                return true;
            }
        }
        false
    };
    let regular = add("regular", sysfonts::REGULAR);
    let bold = add("bold", sysfonts::BOLD);
    let sym = add("symbols", sysfonts::SYMBOLS);
    let mono = add("mono", sysfonts::MONO);
    let han = cjk && add("han", sysfonts::HAN);
    let han_bold = han && add("han_bold", sysfonts::HAN_BOLD);
    let chain = |first: &[(&str, bool)]| -> Vec<String> { first.iter().filter(|(_, ok)| *ok).map(|(k, _)| k.to_string()).collect() };
    fonts.families.insert(FontFamily::Proportional, chain(&[("regular", regular), ("symbols", sym), ("han", han)]));
    fonts.families.insert(bold_family(), chain(&[("bold", bold), ("regular", regular), ("symbols", sym), ("han_bold", han_bold), ("han", han)]));
    fonts.families.insert(FontFamily::Monospace, chain(&[("mono", mono), ("regular", regular), ("symbols", sym), ("han", han)]));
    ctx.set_fonts(fonts);
}

/// Caractère chinois/japonais/coréen : demande le chargement de la police correspondante.
fn is_cjk(c: char) -> bool {
    matches!(c as u32, 0x2E80..=0x9FFF | 0xAC00..=0xD7AF | 0xF900..=0xFAFF | 0xFF00..=0xFFEF)
}

fn setup_style(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();
    v.panel_fill = BG;
    v.window_fill = CARD;
    v.extreme_bg_color = FIELD;
    v.override_text_color = Some(TXT);
    v.selection.bg_fill = ACCENT.gamma_multiply(0.6);
    v.widgets.noninteractive.bg_fill = CARD;
    ctx.set_visuals(v);
    ctx.global_style_mut(|s| {
        s.spacing.item_spacing = vec2(8.0, 8.0);
        s.spacing.scroll.bar_width = 8.0;
        s.interaction.selectable_labels = false; // le texte statique ne doit pas se sélectionner à la souris
    });
}

// ---------- texte mêlant arabe, latin et chiffres ----------
// La bibliothèque d'interface ne gère pas l'ordre bidirectionnel : dans une phrase arabe, les mots latins et les
// nombres s'afficheraient à l'envers. On découpe donc le texte en segments de même sens d'écriture et on les
// place nous-mêmes (de droite à gauche pour l'arabe).
fn is_rtl_char(c: char) -> bool {
    matches!(c as u32, 0x0590..=0x08FF | 0xFB1D..=0xFDFF | 0xFE70..=0xFEFF)
}

/// Symboles d'interface (flèches, formes, pictogrammes, + ×) : toujours placés comme du latin, jamais retournés.
fn is_symbol(c: char) -> bool {
    matches!(c as u32, 0x2190..=0x2BFF | 0x00D7) || c == '+'
}

/// Segments (texte, vrai si de droite à gauche) dans l'ordre logique. Un signe neutre entre deux segments de même
/// sens suit ce sens ; entre deux sens différents, il suit le sens de base de la phrase (celui du premier mot).
fn bidi_runs(text: &str) -> Vec<(String, bool)> {
    let chars: Vec<char> = text.chars().collect();
    let strong: Vec<Option<bool>> = chars
        .iter()
        .map(|&c| if is_rtl_char(c) { Some(true) } else if c.is_alphanumeric() || is_symbol(c) { Some(false) } else { None })
        .collect();
    let base = strong.iter().flatten().next().copied().unwrap_or(false);
    let mut dirs: Vec<bool> = vec![base; chars.len()];
    let mut i = 0;
    while i < chars.len() {
        if let Some(d) = strong[i] {
            dirs[i] = d;
            i += 1;
            continue;
        }
        let start = i;
        while i < chars.len() && strong[i].is_none() {
            i += 1;
        }
        let prev = if start > 0 { strong[..start].iter().rev().flatten().next().copied() } else { None };
        let next = strong[i..].iter().flatten().next().copied();
        let d = if prev.is_some() && prev == next { prev.unwrap_or(base) } else { base };
        for slot in dirs.iter_mut().take(i).skip(start) {
            *slot = d;
        }
    }
    let mut runs: Vec<(String, bool)> = Vec::new();
    for (c, d) in chars.into_iter().zip(dirs) {
        match runs.last_mut() {
            Some((t, rd)) if *rd == d => t.push(c),
            _ => runs.push((c.to_string(), d)),
        }
    }
    runs
}

/// Vrai si ce texte doit être dessiné segment par segment (interface en arabe avec du latin, des chiffres ou des symboles).
fn needs_mixed(text: &str) -> bool {
    i18n::lang() == Lang::Ar && text.chars().any(is_rtl_char) && bidi_runs(text).len() > 1
}

/// Segment prêt à dessiner : espaces de bord (que la mise en page supprime) comptés à part.
struct Run {
    lead: f32,
    trail: f32,
    galley: std::sync::Arc<egui::Galley>,
}

fn layout_runs(painter: &egui::Painter, text: &str, font: &FontId, color: Color32) -> (Vec<Run>, bool) {
    let space = painter.layout_no_wrap("a a".into(), font.clone(), color).size().x - painter.layout_no_wrap("aa".into(), font.clone(), color).size().x;
    let runs = bidi_runs(text);
    let rtl_base = runs.first().is_some_and(|r| r.1);
    let out = runs
        .into_iter()
        .map(|(t, _)| {
            let lead = t.chars().take_while(|c| *c == ' ').count() as f32 * space;
            let trail = t.chars().rev().take_while(|c| *c == ' ').count() as f32 * space;
            Run { lead, trail, galley: painter.layout_no_wrap(t.trim().to_string(), font.clone(), color) }
        })
        .collect();
    (out, rtl_base)
}

fn text_width(painter: &egui::Painter, text: &str, font: &FontId) -> f32 {
    if needs_mixed(text) {
        layout_runs(painter, text, font, TXT).0.iter().map(|r| r.lead + r.galley.size().x + r.trail).sum()
    } else {
        painter.layout_no_wrap(text.to_string(), font.clone(), TXT).size().x
    }
}

/// Comme `Painter::text`, mais gère les textes arabes contenant du latin, des chiffres ou des symboles.
fn paint_text(painter: &egui::Painter, anchor: Align2, pos: egui::Pos2, text: &str, font: FontId, color: Color32) {
    if !needs_mixed(text) {
        painter.text(pos, anchor, text, font, color);
        return;
    }
    let (runs, rtl_base) = layout_runs(painter, text, &font, color);
    let total: f32 = runs.iter().map(|r| r.lead + r.galley.size().x + r.trail).sum();
    let height = runs.iter().map(|r| r.galley.size().y).fold(0.0, f32::max);
    let left = match anchor.x() {
        Align::Min => pos.x,
        Align::Center => pos.x - total / 2.0,
        Align::Max => pos.x - total,
    };
    let top = match anchor.y() {
        Align::Min => pos.y,
        Align::Center => pos.y - height / 2.0,
        Align::Max => pos.y - height,
    };
    // phrase arabe : le premier segment logique est le plus à droite
    let mut x = if rtl_base { left + total } else { left };
    for r in runs {
        let w = r.galley.size().x;
        if rtl_base {
            x -= r.lead + w;
            painter.galley(pos2(x, top), r.galley, color);
            x -= r.trail;
        } else {
            x += r.lead;
            painter.galley(pos2(x, top), r.galley, color);
            x += w + r.trail;
        }
    }
}

/// Étiquette sur une ligne tronquée à la largeur disponible, texte complet au survol.
fn label_truncated(ui: &mut Ui, text: &str, size: f32, color: Color32) {
    if !needs_mixed(text) {
        ui.add(egui::Label::new(RichText::new(text).font(fid(size)).color(color)).truncate()).on_hover_text(text);
        return;
    }
    let font = fid(size);
    let h = ui.painter().layout_no_wrap("A".into(), font.clone(), color).size().y;
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), h), Sense::hover());
    let rtl = bidi_runs(text).first().is_some_and(|r| r.1);
    let (align, anchor_x) = if rtl { (Align2::RIGHT_CENTER, rect.right()) } else { (Align2::LEFT_CENTER, rect.left()) };
    paint_text(&ui.painter().with_clip_rect(rect), align, pos2(anchor_x, rect.center().y), text, font, color);
    resp.on_hover_text(text);
}

// ---------- petits composants dessinés à la main ----------
struct Pill {
    text: String,
    base: Color32,
    hover: Color32,
    fg: Color32,
    size: f32,
    h: f32,
    min_w: f32,
    pad: f32,
    radius: u8,
    enabled: bool,
    full: bool,
    left: bool,
}

impl Pill {
    fn new(text: impl Into<String>, base: Color32, hover: Color32, fg: Color32) -> Self {
        Pill { text: text.into(), base, hover, fg, size: 13.0, h: 36.0, min_w: 0.0, pad: 14.0, radius: 10, enabled: true, full: false, left: false }
    }
    fn size(mut self, s: f32) -> Self {
        self.size = s;
        self
    }
    fn h(mut self, h: f32) -> Self {
        self.h = h;
        self
    }
    fn min_w(mut self, w: f32) -> Self {
        self.min_w = w;
        self
    }
    fn radius(mut self, r: u8) -> Self {
        self.radius = r;
        self
    }
    fn enabled(mut self, e: bool) -> Self {
        self.enabled = e;
        self
    }
    fn full(mut self) -> Self {
        self.full = true;
        self.left = true;
        self
    }
    fn show(self, ui: &mut Ui) -> Response {
        let font = fid_b(self.size);
        let text_w = text_width(ui.painter(), &self.text, &font);
        let w = if self.full { ui.available_width() } else { (text_w + 2.0 * self.pad).max(self.min_w) };
        let (rect, resp) = ui.allocate_exact_size(vec2(w, self.h), if self.enabled { Sense::click() } else { Sense::hover() });
        reg(self.text.clone(), rect);
        let fill = if !self.enabled {
            self.base.gamma_multiply(0.35)
        } else if resp.is_pointer_button_down_on() {
            self.base
        } else if resp.hovered() {
            self.hover
        } else {
            self.base
        };
        let fg = if self.enabled { self.fg } else { self.fg.gamma_multiply(0.45) };
        ui.painter().rect_filled(rect, CornerRadius::same(self.radius), fill);
        if self.left {
            paint_text(ui.painter(), Align2::LEFT_CENTER, pos2(rect.left() + self.pad, rect.center().y), &self.text, font, fg);
        } else {
            paint_text(ui.painter(), Align2::CENTER_CENTER, rect.center(), &self.text, font, fg);
        }
        if resp.hovered() && self.enabled {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        resp
    }
}

fn plain_button(ui: &mut Ui, text: &str) -> Response {
    Pill::new(text, KEY, KEY_HOV, TXT).show(ui)
}

fn card<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    Frame::new()
        .fill(CARD)
        .corner_radius(CornerRadius::same(16))
        .inner_margin(Margin::same(12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner
}

fn label(ui: &mut Ui, text: &str, size: f32, color: Color32) {
    if needs_mixed(text) {
        label_truncated(ui, text, size, color);
    } else {
        ui.label(RichText::new(text).font(fid(size)).color(color));
    }
}

/// Étiquette centrée verticalement dans une ligne de 36 (alignée avec les champs de saisie).
fn vlabel(ui: &mut Ui, text: &str, size: f32, color: Color32) {
    let font = fid(size);
    let w = text_width(ui.painter(), text, &font);
    let (rect, _) = ui.allocate_exact_size(vec2(w + 2.0, 36.0), Sense::hover());
    paint_text(ui.painter(), Align2::LEFT_CENTER, pos2(rect.left(), rect.center().y), text, font, color);
}

/// Interrupteur façon « switch » (toute la zone est cliquable) ; renvoie vrai si la valeur a changé.
fn toggle(ui: &mut Ui, on: &mut bool, text: &str) -> bool {
    let font = fid(13.0);
    let text_w = text_width(ui.painter(), text, &font);
    let (rect, resp) = ui.allocate_exact_size(vec2(40.0 + 8.0 + text_w + 2.0, 36.0), Sense::click());
    reg(text, rect);
    let mut changed = false;
    if resp.clicked() {
        *on = !*on;
        changed = true;
    }
    if resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    let track = Rect::from_center_size(pos2(rect.left() + 20.0, rect.center().y), vec2(40.0, 22.0));
    let t = ui.ctx().animate_bool(resp.id, *on);
    ui.painter().rect_filled(track, CornerRadius::same(11), GRIS.lerp_to_gamma(ACCENT, t));
    let x = egui::lerp(track.left() + 11.0..=track.right() - 11.0, t);
    ui.painter().circle_filled(pos2(x, track.center().y), 8.0, if *on { Color32::WHITE } else { hex(0xd0d4e4) });
    paint_text(ui.painter(), Align2::LEFT_CENTER, pos2(rect.left() + 48.0, rect.center().y), text, font, TXT);
    changed
}

/// Champ numérique : clic pour taper, glisser ou Ctrl + molette pour changer la valeur.
#[allow(clippy::too_many_arguments)]
fn num_field(ui: &mut Ui, v: &mut u32, lo: u32, hi: u32, width: f32, digits: usize, wrap: bool, step: u32, size: f32, enabled: bool) -> Rect {
    let mut field_rect = Rect::NOTHING;
    ui.scope(|ui| {
        {
            let vis = &mut ui.style_mut().visuals;
            for w in [&mut vis.widgets.inactive, &mut vis.widgets.hovered, &mut vis.widgets.active] {
                w.bg_fill = FIELD;
                w.weak_bg_fill = FIELD;
                w.corner_radius = CornerRadius::same(10);
                w.fg_stroke.color = TXT;
                w.bg_stroke = Stroke::new(1.0, BORDER);
            }
        }
        ui.style_mut().override_font_id = Some(fid_m(size));
        let mut f = *v as f64;
        let dv = egui::DragValue::new(&mut f)
            .range(lo as f64..=hi as f64)
            .speed(step.max(1) as f64 * 0.12)
            .custom_formatter(move |n, _| format!("{:0>d$}", n.round() as i64, d = digits))
            .custom_parser(|s| s.trim().parse::<f64>().ok());
        let resp = ui.add_enabled(enabled, |ui: &mut Ui| ui.add_sized(vec2(width, 36.0), dv));
        field_rect = resp.rect;
        let mut nv = f.round().clamp(lo as f64, hi as f64) as u32;
        if enabled && resp.hovered() {
            // Ctrl + molette : egui l'interprète comme un zoom, on lit donc l'événement brut
            let dy: f32 = ui.input(|i| {
                i.events
                    .iter()
                    .filter_map(|e| match e {
                        egui::Event::MouseWheel { delta, modifiers, .. } if modifiers.ctrl => Some(delta.y),
                        _ => None,
                    })
                    .sum()
            });
            if dy != 0.0 {
                let delta = if dy > 0.0 { step as i64 } else { -(step as i64) };
                let span = (hi - lo + 1) as i64;
                let raw = nv as i64 + delta;
                nv = if wrap { ((raw - lo as i64).rem_euclid(span) + lo as i64) as u32 } else { raw.clamp(lo as i64, hi as i64) as u32 };
                ui.input_mut(|i| i.events.retain(|e| !matches!(e, egui::Event::MouseWheel { modifiers, .. } if modifiers.ctrl)));
            }
        }
        *v = nv;
    });
    field_rect
}

/// Liste déroulante au style de l'application ; renvoie le rectangle du bouton.
fn combo(ui: &mut Ui, id: &str, selected: &str, width: f32, add: impl FnOnce(&mut Ui)) -> Rect {
    let mut rect = Rect::NOTHING;
    ui.scope(|ui| {
        {
            let vis = &mut ui.style_mut().visuals;
            for w in [&mut vis.widgets.inactive, &mut vis.widgets.hovered, &mut vis.widgets.active, &mut vis.widgets.open] {
                w.bg_fill = FIELD;
                w.weak_bg_fill = FIELD;
                w.corner_radius = CornerRadius::same(10);
                w.bg_stroke = Stroke::new(1.0, BORDER);
                w.fg_stroke.color = TXT;
            }
            vis.widgets.hovered.weak_bg_fill = KEY_HOV;
            vis.selection.bg_fill = ACCENT.gamma_multiply(0.6);
        }
        ui.style_mut().spacing.button_padding = vec2(10.0, 8.0);
        let r = egui::ComboBox::from_id_salt(id)
            .width(width)
            .selected_text(RichText::new(selected).font(fid(13.0)).color(TXT))
            .show_ui(ui, add);
        rect = r.response.rect;
    });
    rect
}

// ---------- application ----------
pub struct App {
    lang: Lang,
    lang_user: Option<Lang>,      // langue choisie par l'utilisateur (sinon : celle de Windows)
    layout: &'static KbLayout,
    layout_user: Option<&'static str>,
    rows: Vec<Vec<KeyDef>>,       // clavier à l'écran, dans la langue et la disposition courantes
    cjk: bool,                    // police chinoise chargée
    ed: Action,
    actions: Vec<Action>,
    list_mode: bool,
    sel: Option<usize>,
    sh: Arc<Shared>,
    opt_open: bool,
    opt_rect: Rect,
    opt_area: Rect,
    fit: u8,
    rows_seen: usize, // nombre de lignes de la liste au dernier réajustement de la fenêtre
    head_h: f32,
    bottom_h: f32,
    content_h: f32,
    started: Instant,
    frames: u32,
    bench: Option<(Instant, Vec<f64>)>, // AUTOKEY_BENCH=1 : mesure le temps d'image pendant des redimensionnements
    state_path: Option<String>, // AUTOKEY_STATE=chemin.json : écrit l'état interne et la position des composants (tests)
    last_state: String,
    shots_dir: Option<String>,  // AUTOKEY_SHOTS=dossier : capture à la demande (fichier req.txt contenant le nom)
    shot_name: Option<String>,
    autopick: bool,                      // AUTOKEY_AUTOPICK=1 : lance le repérage de zone dès le démarrage (tests)
    autoarm: Option<String>,             // AUTOKEY_AUTOARM=N ou HH:MM:SS : arme l'action dès le démarrage (tests)
    shot: Option<String>, // AUTOKEY_SHOT=chemin.png : enregistre une capture de la fenêtre puis quitte (tests)
}

const TEXT_ID: &str = "texte_a_envoyer";

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let saved = model::load();
        let lang_user = saved.lang.as_deref().and_then(Lang::from_code);
        let lang = std::env::var("AUTOKEY_LANG").ok().and_then(|v| Lang::from_code(&v)).or(lang_user).unwrap_or_else(i18n::detect);
        i18n::set_lang(lang); // avant tout texte : le message d'état initial est dans la bonne langue
        let layout_user = saved.layout.as_deref().and_then(|id| LAYOUTS.iter().find(|l| l.id == id)).map(|l| l.id);
        let layout = if std::env::var_os("AUTOKEY_LAYOUT").is_some() { keys::detect() } else { layout_user.map(keys::by_id).unwrap_or_else(keys::detect) };
        setup_fonts(&cc.egui_ctx, lang == Lang::Zh);
        setup_style(&cc.egui_ctx);
        egui_extras::install_image_loaders(&cc.egui_ctx);
        DEV.store(std::env::var_os("AUTOKEY_STATE").is_some(), Ordering::Relaxed);
        App {
            lang,
            lang_user,
            layout,
            layout_user,
            rows: keys::build(layout),
            cjk: lang == Lang::Zh,
            ed: saved.editor,
            actions: saved.actions,
            list_mode: saved.list_mode,
            sel: None,
            sh: Shared::new(),
            opt_open: false,
            opt_rect: Rect::NOTHING,
            opt_area: Rect::NOTHING,
            fit: 4,
            rows_seen: 0,
            head_h: 70.0,
            bottom_h: 70.0,
            content_h: 500.0,
            started: Instant::now(),
            frames: 0,
            bench: std::env::var("AUTOKEY_BENCH").ok().map(|_| (Instant::now(), vec![])),
            state_path: std::env::var("AUTOKEY_STATE").ok(),
            last_state: String::new(),
            shots_dir: std::env::var("AUTOKEY_SHOTS").ok(),
            shot_name: None,
            autopick: std::env::var("AUTOKEY_AUTOPICK").is_ok(),
            autoarm: std::env::var("AUTOKEY_AUTOARM").ok(),
            shot: std::env::var("AUTOKEY_SHOT").ok(),
        }
    }

    /// Change la langue de l'interface (textes, clavier, polices) sans redémarrer.
    fn set_language(&mut self, ctx: &egui::Context, l: Lang) {
        self.lang = l;
        self.lang_user = Some(l);
        i18n::set_lang(l);
        if l == Lang::Zh && !self.cjk {
            self.cjk = true;
            setup_fonts(ctx, true);
        }
        self.rows = keys::build(self.layout);
        let kind = self.sh.status.lock().unwrap().1;
        if (kind == Kind::Info || engine::session_warning().is_some()) && !self.running() {
            let (text, kind) = worker::idle_status();
            self.sh.set_status(ctx, text, kind);
        }
        self.fit = 4;
        self.persist();
    }

    fn set_layout(&mut self, l: &'static KbLayout) {
        self.layout = l;
        self.layout_user = Some(l.id);
        self.rows = keys::build(l);
        self.persist();
    }

    fn running(&self) -> bool {
        self.sh.running.load(Ordering::SeqCst)
    }

    /// Un envoi ou un repérage de zone est en cours : les réglages sont verrouillés.
    fn busy(&self) -> bool {
        self.running() || self.sh.picking.load(Ordering::SeqCst)
    }

    fn persist(&self) {
        model::save(&Saved {
            editor: self.ed.clone(),
            list_mode: self.list_mode,
            actions: self.actions.clone(),
            lang: self.lang_user.map(|l| l.code().to_string()),
            layout: self.layout_user.map(str::to_string),
        });
    }

    fn status(&self, ctx: &egui::Context, text: impl Into<String>, kind: Kind) {
        self.sh.set_status(ctx, text, kind);
    }

    // ----- saisie -----
    fn insert_text(&mut self, ctx: &egui::Context, s: &str) {
        let id = Id::new(TEXT_ID);
        let mut state = TextEditState::load(ctx, id).unwrap_or_default();
        let chars: Vec<char> = self.ed.text.chars().collect();
        let idx = state.cursor.char_range().map(|r| r.primary.index.0).unwrap_or(chars.len()).min(chars.len());
        let mut out: String = chars[..idx].iter().collect();
        out.push_str(s);
        out.extend(chars[idx..].iter());
        self.ed.text = out;
        state.cursor.set_char_range(Some(CCursorRange::one(CCursor::new(idx + s.chars().count()))));
        state.store(ctx, id);
        ctx.memory_mut(|m| m.request_focus(id));
    }

    /// Place le curseur du champ texte à la fin (après le chargement d'une autre action).
    fn cursor_to_end(&self, ctx: &egui::Context) {
        let id = Id::new(TEXT_ID);
        let mut state = TextEditState::load(ctx, id).unwrap_or_default();
        state.cursor.set_char_range(Some(CCursorRange::one(CCursor::new(self.ed.text.chars().count()))));
        state.store(ctx, id);
    }

    fn click_key(&mut self, ctx: &egui::Context, key: KeyDef) {
        if self.busy() {
            return;
        }
        match key.tok {
            Tok::Mod(m) => {
                let name = m.canonical();
                if let Some(p) = self.ed.mods.iter().position(|x| x == name) {
                    self.ed.mods.remove(p);
                } else {
                    self.ed.mods.push(name.to_string());
                }
            }
            Tok::Named(n) => self.insert_text(ctx, &format!("[{}]", n.canonical())),
            Tok::Char(c) => self.insert_text(ctx, &format!("[{c}]")),
            Tok::Text(t) => self.insert_text(ctx, &t),
        }
    }

    // ----- actions -----
    /// Valide les réglages actuels ; sinon affiche la raison dans la barre d'état.
    fn snapshot(&self, ctx: &egui::Context, check_date: bool) -> Option<Action> {
        let a = self.ed.clone();
        if let Err(e) = model::parse(&a.text) {
            self.status(ctx, e, Kind::Err);
            return None;
        }
        if a.text.is_empty() && a.mods.is_empty() {
            self.status(ctx, i18n::t(Msg::NothingToSend), Kind::Warn);
            return None;
        }
        if check_date {
            if let Err(e) = model::when(&a) {
                self.status(ctx, tf(Msg::InvalidDateTime, &[&e]), Kind::Err);
                return None;
            }
        }
        Some(a)
    }

    fn arm(&mut self, ctx: &egui::Context) {
        if self.busy() {
            return;
        }
        let jobs: Result<Vec<model::Job>, String> = if self.list_mode {
            if self.actions.is_empty() {
                self.status(ctx, i18n::t(Msg::ListEmpty), Kind::Warn);
                return;
            }
            self.actions
                .iter()
                .enumerate()
                .map(|(i, a)| model::make_job(a, None).map_err(|e| tf(Msg::ActionInvalid, &[&(i + 1), &e])))
                .collect()
        } else {
            let Some(a) = self.snapshot(ctx, true) else { return };
            model::make_job(&a, None).map(|j| vec![j])
        };
        match jobs {
            Ok(mut jobs) => {
                jobs.sort_by(|a, b| a.ts.total_cmp(&b.ts));
                self.persist();
                worker::spawn_jobs(jobs, None, self.sh.clone(), ctx.clone());
            }
            Err(e) => self.status(ctx, e, Kind::Err),
        }
    }

    fn test(&mut self, ctx: &egui::Context) {
        if self.busy() {
            return;
        }
        let Some(a) = self.snapshot(ctx, false) else { return };
        match model::make_job(&a, Some(model::now_secs() + 3.0)) {
            Ok(job) => worker::spawn_jobs(vec![job], Some(i18n::t(Msg::TestName).to_string()), self.sh.clone(), ctx.clone()),
            Err(e) => self.status(ctx, e, Kind::Err),
        }
    }

    fn start_pick(&mut self, ctx: &egui::Context) {
        if self.busy() {
            return;
        }
        worker::spawn_pick(self.sh.clone(), ctx.clone());
        ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
    }

    fn restore_window(ctx: &egui::Context) {
        ctx.send_viewport_cmd(ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(ViewportCommand::Focus);
    }

    // ----- parties de l'interface -----
    fn head(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        ui.horizontal(|ui| {
            ui.add(egui::Image::new(egui::include_image!("../assets/autokey.png")).fit_to_exact_size(vec2(40.0, 40.0)));
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.label(RichText::new("AutoKey").font(fid_b(22.0)).color(TXT));
                label(ui, i18n::t(Msg::Subtitle), 11.0, MUTED);
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let before = self.list_mode;
                toggle(ui, &mut self.list_mode, i18n::t(Msg::ListMode));
                if before != self.list_mode {
                    self.fit = 4;
                    self.persist();
                }
                ui.add_space(14.0);
                let who = AUTHOR.split(" aka ").next().unwrap_or("");
                if Pill::new(format!("♥  {}", tf(Msg::Donate, &[&who])), PAYPAL, PAYPAL_HOV, Color32::WHITE).radius(18).show(ui).clicked() {
                    engine::open_url(DONATE_URL);
                }
                ui.add_space(8.0);
                let mut chosen: Option<Lang> = None;
                let r = combo(ui, "lang_combo", self.lang.name(), 120.0, |ui| {
                    for l in Lang::ALL {
                        let it = ui.selectable_label(l == self.lang, RichText::new(l.name()).font(fid(14.0)));
                        reg(format!("lang:{}", l.code()), it.rect);
                        if it.clicked() {
                            chosen = Some(l);
                        }
                    }
                });
                reg("lang", r);
                if let Some(l) = chosen {
                    self.set_language(ctx, l);
                }
            });
        });
    }

    /// Barre au-dessus du clavier : choix de la disposition.
    fn layout_bar(&mut self, ui: &mut Ui) {
        let busy = self.busy();
        let mut chosen: Option<&'static KbLayout> = None;
        ui.horizontal(|ui| {
            vlabel(ui, i18n::t(Msg::Keyboard), 12.0, MUTED);
            ui.add_enabled_ui(!busy, |ui| {
                let r = combo(ui, "layout_combo", self.layout.name, 190.0, |ui| {
                    for l in LAYOUTS {
                        let it = ui.selectable_label(l.id == self.layout.id, RichText::new(l.name).font(fid(14.0)));
                        reg(format!("layout:{}", l.id), it.rect);
                        if it.clicked() {
                            chosen = Some(l);
                        }
                    }
                });
                reg("layout", r);
            });
        });
        if let Some(l) = chosen {
            self.set_layout(l);
        }
    }

    fn keyboard(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        let (kh, gap) = (30.0_f32, 3.0_f32);
        let avail = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(vec2(avail, self.rows.len() as f32 * (kh + gap)), Sense::hover());
        let mut clicked: Option<KeyDef> = None;
        for (r, row) in self.rows.iter().enumerate() {
            let total: f32 = row.iter().map(|k| k.w).sum();
            let mut cum = 0.0;
            let y0 = rect.top() + r as f32 * (kh + gap) + gap / 2.0;
            for (i, key) in row.iter().enumerate() {
                let x0 = rect.left() + cum / total * avail + gap / 2.0;
                let x1 = rect.left() + (cum + key.w) / total * avail - gap / 2.0;
                cum += key.w;
                let kr = Rect::from_min_max(pos2(x0, y0), pos2(x1, y0 + kh));
                let resp = ui.interact(kr, ui.id().with(("key", r, i)), Sense::click());
                reg(format!("touche:{}", key.label), kr);
                let held = matches!(key.tok, Tok::Mod(m) if self.ed.mods.iter().any(|x| x == m.canonical()));
                let fill = if held { ORANGE } else if resp.hovered() { KEY_HOV } else { KEY };
                ui.painter().rect_filled(kr, CornerRadius::same(8), fill);
                ui.painter().text(kr.center(), Align2::CENTER_CENTER, &key.label, fid_b(12.5), if held { DARK } else { TXT });
                if resp.hovered() {
                    ctx.set_cursor_icon(CursorIcon::PointingHand);
                }
                if resp.clicked() {
                    clicked = Some(key.clone());
                }
            }
        }
        if let Some(k) = clicked {
            self.click_key(ctx, k);
        }
    }

    fn programming(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        let running = self.running();
        ui.horizontal_wrapped(|ui| {
            ui.add_enabled_ui(!running, |ui| {
                vlabel(ui, i18n::t(Msg::Time), 13.0, MUTED);
                reg("num:h", num_field(ui, &mut self.ed.h, 0, 23, 52.0, 2, true, 1, 18.0, true));
                vlabel(ui, ":", 18.0, MUTED);
                reg("num:m", num_field(ui, &mut self.ed.m, 0, 59, 52.0, 2, true, 1, 18.0, true));
                vlabel(ui, ":", 18.0, MUTED);
                reg("num:s", num_field(ui, &mut self.ed.s, 0, 59, 52.0, 2, true, 1, 18.0, true));
                vlabel(ui, ".", 18.0, MUTED);
                reg("num:ms", num_field(ui, &mut self.ed.ms, 0, 999, 66.0, 3, true, 50, 18.0, true));
                vlabel(ui, i18n::t(Msg::Ms), 12.0, MUTED);
                ui.add_space(10.0);
                toggle(ui, &mut self.ed.use_date, i18n::t(Msg::Date));
                ui.scope(|ui| {
                    ui.style_mut().visuals.extreme_bg_color = FIELD;
                    let date_resp = ui.add_enabled(
                        self.ed.use_date,
                        egui::TextEdit::singleline(&mut self.ed.date)
                            .font(fid_m(16.0))
                            .desired_width(104.0)
                            .margin(Margin::symmetric(8, 8)),
                    );
                    reg("date", date_resp.rect);
                });
                ui.add_space(10.0);
                toggle(ui, &mut self.ed.repeat, i18n::t(Msg::Repeat));
                reg("num:rep", num_field(ui, &mut self.ed.rep, 2, 9999, 60.0, 0, false, 1, 15.0, self.ed.repeat));
                vlabel(ui, i18n::t(Msg::EveryX), 12.0, MUTED);
                reg("num:gap", num_field(ui, &mut self.ed.gap, 0, 60000, 70.0, 0, false, 50, 15.0, self.ed.repeat));
                vlabel(ui, i18n::t(Msg::Ms), 12.0, MUTED);
            });
        });
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.add_enabled_ui(!running, |ui| {
                if Pill::new(format!("◎  {}", i18n::t(Msg::PickTarget)), ACCENT, ACCENT_HOV, Color32::WHITE).show(ui).clicked() {
                    self.start_pick(ctx);
                }
                if Pill::new("✕", KEY, KEY_HOV, TXT).min_w(36.0).show(ui).clicked() {
                    self.ed.target = None;
                }
                let r = Pill::new(format!("⚙  {}  ▾", i18n::t(Msg::Options)), KEY, KEY_HOV, TXT).show(ui);
                self.opt_rect = r.rect;
                if r.clicked() {
                    self.opt_open = !self.opt_open;
                }
            });
            for (on, color) in [(self.ed.minim, VIOLET), (self.ed.use_target, JAUNE), (self.ed.back, ORANGE)] {
                let (rect, _) = ui.allocate_exact_size(vec2(20.0, 20.0), Sense::hover());
                ui.painter().rect_filled(rect, CornerRadius::same(6), if on { color } else { GRIS });
            }
            let text = match &self.ed.target {
                Some(t) => tf(Msg::TargetInfo, &[&t.exe, &t.title.chars().take(70).collect::<String>(), &t.dx, &t.dy]),
                None => i18n::t(Msg::NoTarget).to_string(),
            };
            label_truncated(ui, &text, 12.0, MUTED);
        });
    }

    fn options_popup(&mut self, ctx: &egui::Context) {
        if !self.opt_open {
            return;
        }
        let h = 3.0 * 42.0 + 4.0 * 8.0 + 12.0;
        let below = self.opt_rect.left_bottom() + vec2(0.0, 6.0);
        let screen = ctx.content_rect();
        let pos = if below.y + h > screen.bottom() - self.bottom_h { pos2(below.x, self.opt_rect.top() - h - 6.0) } else { below };
        let defs = [
            (i18n::t(Msg::OptMinimize), VIOLET, 0),
            (i18n::t(Msg::OptGoTarget), JAUNE, 1),
            (i18n::t(Msg::OptGoBack), ORANGE, 2),
        ];
        let out = egui::Area::new(Id::new("options_popup")).order(egui::Order::Foreground).fixed_pos(pos).show(ctx, |ui| {
            Frame::new()
                .fill(CARD)
                .stroke(Stroke::new(1.0, BORDER))
                .corner_radius(CornerRadius::same(14))
                .inner_margin(Margin::same(8))
                .show(ui, |ui| {
                    ui.set_width(300.0);
                    for (text, color, which) in defs {
                        let on = match which {
                            0 => self.ed.minim,
                            1 => self.ed.use_target,
                            _ => self.ed.back,
                        };
                        let (base, hover, fg) = if on { (color, color, DARK) } else { (KEY, KEY_HOV, TXT) };
                        let label = format!("{}   {}", if on { "✔" } else { "☐" }, text);
                        if Pill::new(label, base, hover, fg).h(42.0).size(13.0).full().show(ui).clicked() {
                            match which {
                                0 => self.ed.minim = !self.ed.minim,
                                1 => self.ed.use_target = !self.ed.use_target,
                                _ => self.ed.back = !self.ed.back,
                            }
                        }
                    }
                });
        });
        self.opt_area = out.response.rect;
        reg("popup", self.opt_area);
        // clic en dehors : ferme le menu
        let (pressed, pos) = ctx.input(|i| (i.pointer.any_pressed(), i.pointer.interact_pos()));
        if pressed && self.started.elapsed().as_millis() > 0 {
            if let Some(p) = pos {
                if !self.opt_area.contains(p) && !self.opt_rect.contains(p) {
                    self.opt_open = false;
                }
            }
        }
    }

    fn what(&mut self, ui: &mut Ui) {
        let running = self.running();
        let text_edit = egui::TextEdit::singleline(&mut self.ed.text)
            .id(Id::new(TEXT_ID))
            .hint_text(RichText::new(i18n::t(Msg::TextHint)).color(hex(0x6b7290)))
            .font(fid_m(16.0))
            .text_color(DARK)
            .desired_width(f32::INFINITY)
            .margin(Margin::symmetric(12, 11))
            .background_color(Color32::WHITE);
        ui.add_enabled_ui(!running, |ui| {
            ui.scope(|ui| {
                let vis = &mut ui.style_mut().visuals;
                for w in [&mut vis.widgets.inactive, &mut vis.widgets.hovered, &mut vis.widgets.active, &mut vis.widgets.noninteractive] {
                    w.corner_radius = CornerRadius::same(12);
                    w.bg_stroke = Stroke::new(2.0, ACCENT);
                }
                vis.selection.stroke = Stroke::new(2.0, ACCENT);
                vis.override_text_color = Some(hex(0x5a6180)); // couleur du texte d'indication (le texte saisi reste sombre)
                let text_resp = ui.add(text_edit);
                reg("texte", text_resp.rect);
            });
        });
        if !self.cjk && self.ed.text.chars().any(is_cjk) {
            self.cjk = true;
            setup_fonts(ui.ctx(), true); // du chinois/japonais/coréen est saisi : on charge la police correspondante
        }
        let mods: Vec<String> = self.ed.mods.iter().filter_map(|m| Modk::parse(m)).map(|k| k.label()).collect();
        let info = if mods.is_empty() { i18n::t(Msg::HintKeyboard).to_string() } else { tf(Msg::HeldKeys, &[&mods.join(" + ")]) };
        label(ui, &info, 12.0, MUTED);
    }

    // ----- liste d'actions -----
    fn list_card(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        label(ui, i18n::t(Msg::ListTitle), 11.0, MUTED);
        label(ui, i18n::t(Msg::ListHelp), 12.0, MUTED);
        let mut load: Option<usize> = None;
        let mut select: Option<usize> = None;
        for (i, a) in self.actions.iter().enumerate() {
            let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 44.0), Sense::click());
            reg(format!("ligne{i}"), rect);
            let fill = if self.sel == Some(i) { ROW_SEL } else { ROW };
            ui.painter().rect_filled(rect, CornerRadius::same(10), fill);
            for (k, (on, color)) in [(a.minim, VIOLET), (a.use_target, JAUNE), (a.back, ORANGE)].into_iter().enumerate() {
                let sq = Rect::from_min_size(pos2(rect.left() + 10.0 + k as f32 * 28.0, rect.center().y - 11.0), vec2(22.0, 22.0));
                ui.painter().rect_filled(sq, CornerRadius::same(6), if on { color } else { GRIS });
            }
            let when = format!(
                "{} {:02}:{:02}:{:02}.{:03}",
                if a.use_date { a.date.as_str() } else { i18n::t(Msg::NextOccurrence) },
                a.h, a.m, a.s, a.ms
            );
            let tgt = match (&a.target, a.use_target) {
                (Some(t), true) => t.exe.clone(),
                _ => i18n::t(Msg::ActiveWindow).to_string(),
            };
            let rep = if a.repeat { format!("×{}", a.rep) } else { "-".to_string() };
            let mods: Vec<String> = a.mods.iter().filter_map(|m| Modk::parse(m)).map(|k| k.label()).collect();
            let txt = match (mods.is_empty(), a.text.is_empty()) {
                (true, true) => i18n::t(Msg::HeldOnly).to_string(),
                (true, false) => a.text.clone(),
                (false, true) => mods.join(" + "),
                (false, false) => format!("{} + {}", mods.join(" + "), a.text),
            };
            let (x_when, x_tgt, x_rep) = (rect.left() + 110.0, rect.right() - 230.0, rect.right() - 70.0);
            let x_text = x_when + 210.0;
            let cell = |x0: f32, x1: f32, text: &str, font: FontId, ui: &Ui| {
                let clip = Rect::from_min_max(pos2(x0, rect.top()), pos2(x1, rect.bottom()));
                paint_text(&ui.painter().with_clip_rect(clip), Align2::LEFT_CENTER, pos2(x0, rect.center().y), text, font, TXT);
            };
            cell(x_when, x_text - 6.0, &when, fid_m(13.0), ui);
            cell(x_text, x_tgt - 6.0, &txt, fid(13.0), ui);
            cell(x_tgt, x_rep - 6.0, &tgt, fid(13.0), ui);
            cell(x_rep, rect.right() - 6.0, &rep, fid(13.0), ui);
            if resp.double_clicked() {
                load = Some(i);
            } else if resp.clicked() {
                select = Some(i);
            }
        }
        if let Some(i) = select {
            self.sel = Some(i);
        }
        if let Some(i) = load {
            self.sel = Some(i);
            if !self.busy() {
                self.ed = self.actions[i].clone();
                self.cursor_to_end(ctx);
            }
        }
        let busy = self.busy();
        let mut changed = false;
        ui.add_enabled_ui(!busy, |ui| {
            ui.horizontal_wrapped(|ui| {
                if plain_button(ui, &format!("+  {}", i18n::t(Msg::AddAction))).clicked() {
                    if let Some(a) = self.snapshot(ctx, true) {
                        self.actions.push(a);
                        self.sel = Some(self.actions.len() - 1);
                        changed = true;
                    }
                }
                if plain_button(ui, &format!("⟳  {}", i18n::t(Msg::ReplaceRow))).clicked() {
                    if let Some(i) = self.sel.filter(|i| *i < self.actions.len()) {
                        if let Some(a) = self.snapshot(ctx, true) {
                            self.actions[i] = a;
                            changed = true;
                        }
                    } else {
                        self.status(ctx, i18n::t(Msg::SelectRowFirst), Kind::Warn);
                    }
                }
                if plain_button(ui, &format!("✕  {}", i18n::t(Msg::Delete))).clicked() {
                    if let Some(i) = self.sel.filter(|i| *i < self.actions.len()) {
                        self.actions.remove(i);
                        self.sel = None;
                        changed = true;
                    } else {
                        self.status(ctx, i18n::t(Msg::SelectRowFirst), Kind::Warn);
                    }
                }
                if plain_button(ui, i18n::t(Msg::ClearAll)).clicked() {
                    self.actions.clear();
                    self.sel = None;
                    changed = true;
                }
            });
        });
        if changed {
            self.persist();
        }
    }

    /// Outils de test pilotés par variables d'environnement (voir README) : sans effet en usage normal.
    fn dev_hooks(&mut self, ctx: &egui::Context) {
        if let Some(path) = self.state_path.clone() {
            let ppp = ctx.pixels_per_point();
            let widgets: serde_json::Map<String, serde_json::Value> = WIDGETS.with(|w| {
                w.borrow().iter().map(|(n, r)| (n.clone(), serde_json::json!([r.min.x * ppp, r.min.y * ppp, r.width() * ppp, r.height() * ppp]))).collect()
            });
            let (status, kind) = self.sh.status.lock().unwrap().clone();
            let state = serde_json::json!({
                "text": self.ed.text, "mods": self.ed.mods, "h": self.ed.h, "m": self.ed.m, "s": self.ed.s, "ms": self.ed.ms,
                "use_date": self.ed.use_date, "date": self.ed.date, "repeat": self.ed.repeat, "rep": self.ed.rep, "gap": self.ed.gap,
                "minim": self.ed.minim, "use_target": self.ed.use_target, "back": self.ed.back,
                "target": self.ed.target.as_ref().map(|t| t.exe.clone()), "list_mode": self.list_mode,
                "actions": self.actions.iter().map(|a| a.text.clone()).collect::<Vec<_>>(), "sel": self.sel,
                "opt_open": self.opt_open, "lang": self.lang.code(), "layout": self.layout.id, "running": self.running(), "picking": self.sh.picking.load(Ordering::SeqCst),
                "status": status, "kind": format!("{kind:?}"), "ppp": ppp, "widgets": widgets,
            })
            .to_string();
            if state != self.last_state {
                let tmp = format!("{path}.tmp");
                if std::fs::write(&tmp, &state).is_ok() {
                    let _ = std::fs::rename(&tmp, &path);
                }
                self.last_state = state;
            }
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        if let Some(dir) = self.shots_dir.clone() {
            let req = std::path::Path::new(&dir).join("req.txt");
            if self.shot_name.is_none() {
                if let Ok(name) = std::fs::read_to_string(&req) {
                    let _ = std::fs::remove_file(&req);
                    self.shot_name = Some(name.trim().to_string());
                    ctx.send_viewport_cmd(ViewportCommand::Screenshot(egui::UserData::default()));
                }
            }
            let shot = ctx.input(|i| {
                i.events.iter().find_map(|e| match e {
                    egui::Event::Screenshot { image, .. } => Some(image.clone()),
                    _ => None,
                })
            });
            if let (Some(img), true) = (shot, self.shot_name.is_some()) {
                let name = self.shot_name.take().unwrap_or_default();
                let rgba: Vec<u8> = img.pixels.iter().flat_map(|c| c.to_array()).collect();
                let _ = image::save_buffer(std::path::Path::new(&dir).join(format!("{name}.png")), &rgba, img.width() as u32, img.height() as u32, image::ColorType::Rgba8);
            }
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        if self.autopick && self.frames == 5 {
            self.start_pick(ctx);
        }
        if let (Some(spec), 5) = (self.autoarm.clone(), self.frames) {
            use chrono::Timelike;
            let t = match spec.split(':').map(str::parse::<u32>).collect::<Result<Vec<_>, _>>().ok().filter(|v| v.len() == 3) {
                Some(hms) => (hms[0], hms[1], hms[2]),
                None => {
                    let t = chrono::Local::now() + chrono::Duration::seconds(spec.parse().unwrap_or(10));
                    (t.hour(), t.minute(), t.second())
                }
            };
            (self.ed.h, self.ed.m, self.ed.s, self.ed.ms) = (t.0, t.1, t.2, 0);
            self.arm(ctx);
        }
        if let Some((last, times)) = self.bench.as_mut() {
            times.push(last.elapsed().as_secs_f64() * 1000.0);
            *last = Instant::now();
            let n = times.len();
            if n < 140 {
                let w = 1000.0 + ((n % 2) as f32) * 60.0 + (n as f32 % 7.0) * 3.0;
                ctx.send_viewport_cmd(ViewportCommand::InnerSize(vec2(w, 640.0 + (n % 3) as f32 * 20.0)));
                ctx.request_repaint();
            } else {
                let mut t: Vec<f64> = times[20..].to_vec();
                t.sort_by(|a, b| a.total_cmp(b));
                let avg = t.iter().sum::<f64>() / t.len() as f64;
                let _ = std::fs::write(
                    std::env::temp_dir().join("autokey_bench.txt"),
                    format!("images={} moyenne={avg:.1}ms mediane={:.1}ms p95={:.1}ms max={:.1}ms", t.len(), t[t.len() / 2], t[t.len() * 95 / 100], t[t.len() - 1]),
                );
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }
        }

        // capture d'écran de test
        self.frames += 1;
        if let Some(path) = self.shot.clone() {
            if self.frames == 15 {
                ctx.send_viewport_cmd(ViewportCommand::Screenshot(egui::UserData::default()));
            }
            let shot = ctx.input(|i| {
                i.events.iter().find_map(|e| match e {
                    egui::Event::Screenshot { image, .. } => Some(image.clone()),
                    _ => None,
                })
            });
            if let Some(img) = shot {
                let rgba: Vec<u8> = img.pixels.iter().flat_map(|c| c.to_array()).collect();
                let _ = image::save_buffer(&path, &rgba, img.width() as u32, img.height() as u32, image::ColorType::Rgba8);
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }
            ctx.request_repaint();
        }
    }

    fn bottom(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        let running = self.running();
        ui.horizontal(|ui| {
            let go = format!("▶  {}", i18n::t(if self.list_mode { Msg::ArmList } else { Msg::Arm }));
            if Pill::new(go, GREEN, GREEN_HOV, hex(0x06210f)).size(14.0).h(40.0).radius(12).min_w(140.0).enabled(!running).show(ui).clicked() {
                self.arm(ctx);
            }
            if Pill::new(format!("■  {}", i18n::t(Msg::Cancel)), RED, RED_HOV, Color32::WHITE).size(14.0).h(40.0).radius(12).min_w(120.0).enabled(running).show(ui).clicked() {
                self.sh.cancel.store(true, Ordering::SeqCst);
            }
            if Pill::new(i18n::t(Msg::Clear), KEY, KEY_HOV, TXT).h(40.0).radius(12).enabled(!running).show(ui).clicked() {
                self.ed.text.clear();
                self.ed.mods.clear();
            }
            if Pill::new(i18n::t(Msg::TestBtn), KEY, KEY_HOV, TXT).h(40.0).radius(12).enabled(!running).show(ui).clicked() {
                self.test(ctx);
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let (text, kind) = self.sh.status.lock().unwrap().clone();
                let color = match kind {
                    Kind::Info => MUTED,
                    Kind::Wait => hex(0x9ab4ff),
                    Kind::Ok => hex(0x5be08b),
                    Kind::Warn => hex(0xffb454),
                    Kind::Err => hex(0xff7b7b),
                };
                label_truncated(ui, &text, 12.0, color);
            });
        });
    }
}

impl eframe::App for App {
    /// Appelé avant chaque image, même fenêtre réduite : réagit aux demandes des threads d'arrière-plan.
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.sh.minimize.swap(false, Ordering::SeqCst) {
            ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
        }
        if self.sh.restore.swap(false, Ordering::SeqCst) {
            Self::restore_window(ctx);
        }
        let picked = self.sh.picked.lock().unwrap().take();
        if let Some(result) = picked {
            Self::restore_window(ctx);
            if let Some(t) = result {
                let t: Target = t;
                self.ed.target = Some(t);
                self.ed.use_target = true;
                self.persist();
            }
        }
    }

    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        WIDGETS.with(|w| w.borrow_mut().clear());

        let head = egui::Panel::top("head")
            .frame(Frame::new().fill(BG).inner_margin(Margin::symmetric(20, 10)))
            .show(ui, |ui| self.head(ui, &ctx));
        self.head_h = head.response.rect.height();

        let bottom = egui::Panel::bottom("bottom")
            .frame(Frame::new().fill(CARD).inner_margin(Margin::symmetric(14, 8)))
            .show(ui, |ui| self.bottom(ui, &ctx));
        self.bottom_h = bottom.response.rect.height();

        egui::CentralPanel::default().frame(Frame::new().fill(BG).inner_margin(Margin::symmetric(10, 4))).show(ui, |ui| {
            let out = ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 10.0;
                card(ui, |ui| {
                    self.layout_bar(ui);
                    self.keyboard(ui, &ctx);
                });
                card(ui, |ui| self.programming(ui, &ctx));
                card(ui, |ui| self.what(ui));
                if self.list_mode {
                    card(ui, |ui| self.list_card(ui, &ctx));
                }
            });
            self.content_h = out.content_size.y;
        });

        self.options_popup(&ctx);

        // sauvegarde des réglages à la fermeture de la fenêtre
        if ctx.input(|i| i.viewport().close_requested()) {
            self.sh.cancel.store(true, Ordering::SeqCst);
            // laisse au thread d'envoi le temps de relâcher les touches Ctrl/Alt/Maj éventuellement enfoncées
            let start = Instant::now();
            while self.running() && start.elapsed() < std::time::Duration::from_millis(500) {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            self.persist();
        }

        self.dev_hooks(&ctx);

        if self.list_mode && self.actions.len() != self.rows_seen {
            self.rows_seen = self.actions.len();
            self.fit = 4;
        }

        // ajuste la hauteur de la fenêtre au contenu (dans la limite de l'écran)
        if self.fit > 0 {
            self.fit -= 1;
            if self.fit == 0 && !ctx.input(|i| i.viewport().maximized.unwrap_or(false)) {
                let w = ctx.content_rect().width();
                let monitor_h = ctx.input(|i| i.viewport().monitor_size.map(|m| m.y)).unwrap_or(900.0);
                let h = (self.head_h + self.content_h + self.bottom_h + 14.0).min(monitor_h - 90.0).max(460.0);
                ctx.send_viewport_cmd(ViewportCommand::InnerSize(vec2(w, h)));
            }
            ctx.request_repaint();
        }
    }
}
