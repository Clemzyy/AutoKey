//! Moteur macOS : envoi de touches et de clics par Quartz (CGEvent), fenêtres par CGWindowList.
//!
//! macOS exige l'autorisation « Accessibilité » (Réglages Système > Confidentialité et sécurité) pour envoyer des touches
//! à d'autres applications, et « Enregistrement de l'écran » pour lire les titres de leurs fenêtres (voir `session_warning`).
use std::ffi::c_void;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread::sleep;
use std::time::Duration;

use core_foundation::base::{CFType, TCFType};
use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
use core_foundation::number::CFNumber;
use core_foundation::string::CFString;
use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation, CGEventType, CGMouseButton};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use core_graphics::geometry::CGPoint;
use core_graphics::window::{copy_window_info, kCGNullWindowID, kCGWindowListExcludeDesktopElements, kCGWindowListOptionAll, kCGWindowListOptionOnScreenOnly};

use super::Target;
use crate::i18n::{self, Msg};

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventSourceKeyState(state: i32, key: u16) -> bool;
    fn CGEventSourceButtonState(state: i32, button: u32) -> bool;
    fn CGWarpMouseCursorPosition(p: CGPoint) -> i32;
}

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> bool;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFDataGetBytePtr(data: *const c_void) -> *const u8;
    fn CFRelease(obj: *const c_void);
}

#[link(name = "Carbon", kind = "framework")]
extern "C" {
    static kTISPropertyUnicodeKeyLayoutData: *const c_void;
    fn TISCopyCurrentKeyboardLayoutInputSource() -> *const c_void;
    fn TISGetInputSourceProperty(source: *const c_void, key: *const c_void) -> *const c_void;
    fn LMGetKbdType() -> u8;
    #[allow(clippy::too_many_arguments)]
    fn UCKeyTranslate(
        layout: *const u8,
        key_code: u16,
        action: u16,
        modifier_state: u32,
        kbd_type: u32,
        options: u32,
        dead_key_state: *mut u32,
        max_len: usize,
        actual_len: *mut usize,
        out: *mut u16,
    ) -> i32;
}

/// État combiné de la session (clavier et souris physiques + événements envoyés).
const COMBINED: i32 = 0;

/// Message à afficher si l'autorisation « Accessibilité » manque.
pub fn session_warning() -> Option<Msg> {
    (!unsafe { AXIsProcessTrusted() }).then_some(Msg::MacPermission)
}

// ---------- envoi de touches ----------
/// Touche virtuelle macOS (clavier ANSI) pour un code de balayage Windows (jeu 1).
fn mac_key(scan: u16, ext: bool) -> Option<u16> {
    const FKEYS: [u16; 10] = [122, 120, 99, 118, 96, 97, 98, 100, 101, 109];
    Some(match (scan, ext) {
        (0x01, false) => 53,
        (0x0E, false) => 51,
        (0x0F, false) => 48,
        (0x1C, false) => 36,
        (0x1D, false) => 59, // Ctrl
        (0x2A, false) => 56,
        (0x36, false) => 60,
        (0x38, false) => 58, // Option
        (0x38, true) => 61,  // Option droite (Alt Gr)
        (0x39, false) => 49,
        (0x3A, false) => 57,
        (n @ 0x3B..=0x44, false) => FKEYS[(n - 0x3B) as usize],
        (0x57, false) => 103,
        (0x58, false) => 111,
        (0x47, true) => 115,
        (0x48, true) => 126,
        (0x49, true) => 116,
        (0x4B, true) => 123,
        (0x4D, true) => 124,
        (0x4F, true) => 119,
        (0x50, true) => 125,
        (0x51, true) => 121,
        (0x52, true) => 114,
        (0x53, true) => 117,
        _ => return None,
    })
}

/// Drapeau de modificateur porté par une touche virtuelle (0 si ce n'est pas un modificateur).
fn modifier_flag(key: u16) -> u64 {
    match key {
        56 | 60 => 0x0002_0000, // Maj
        59 | 62 => 0x0004_0000, // Contrôle
        58 | 61 => 0x0008_0000, // Option
        55 | 54 => 0x0010_0000, // Commande
        _ => 0,
    }
}

/// Modificateurs actuellement maintenus par AutoKey : macOS veut les drapeaux sur chaque événement.
static HELD: AtomicU64 = AtomicU64::new(0);

fn post_key(key: u16, down: bool, extra_flags: u64, text: Option<&str>) -> bool {
    let Ok(source) = CGEventSource::new(CGEventSourceStateID::HIDSystemState) else { return false };
    let Ok(event) = CGEvent::new_keyboard_event(source, key, down) else { return false };
    event.set_flags(CGEventFlags::from_bits_truncate(HELD.load(Ordering::SeqCst) | extra_flags));
    if let Some(t) = text {
        event.set_string(t);
    }
    event.post(CGEventTapLocation::HID);
    true
}

/// Appuie (up = false) ou relâche (up = true) une touche physique, par code de balayage.
pub fn send_scan(scan: u16, ext: bool, up: bool) -> bool {
    let Some(key) = mac_key(scan, ext) else { return false };
    let flag = modifier_flag(key);
    if flag != 0 {
        if up {
            HELD.fetch_and(!flag, Ordering::SeqCst);
        } else {
            HELD.fetch_or(flag, Ordering::SeqCst);
        }
    }
    post_key(key, !up, 0, None)
}

/// Tape un caractère quelconque (accents, symboles…), indépendamment de la disposition du clavier.
pub fn send_char(c: char) -> bool {
    let s = c.to_string();
    post_key(0, true, 0, Some(&s)) & post_key(0, false, 0, Some(&s))
}

/// Touches du pavé numérique : à éviter, leur effet dépend du verrouillage numérique.
fn is_keypad(key: u16) -> bool {
    matches!(key, 65 | 67 | 69 | 71 | 75 | 76 | 78 | 81..=92)
}

/// (touche, Maj, Option) qui produit `c` avec la disposition de clavier active ; `None` si elle ne le contient pas.
fn find_char(c: char) -> Option<(u16, bool, bool)> {
    let mut want = [0u16; 2];
    let units = c.encode_utf16(&mut want);
    if units.len() != 1 {
        return None;
    }
    let target = units[0];
    unsafe {
        let source = TISCopyCurrentKeyboardLayoutInputSource();
        if source.is_null() {
            return None;
        }
        let data = TISGetInputSourceProperty(source, kTISPropertyUnicodeKeyLayoutData);
        let layout = if data.is_null() { std::ptr::null() } else { CFDataGetBytePtr(data) };
        let mut found = None;
        if !layout.is_null() {
            let kbd = LMGetKbdType() as u32;
            'search: for (shift, option) in [(false, false), (true, false), (false, true), (true, true)] {
                let state = (shift as u32) * 2 + (option as u32) * 8; // (modificateurs Carbon) >> 8
                let mut best: Option<(bool, u16)> = None;
                for key in 0u16..128 {
                    let (mut dead, mut len, mut out) = (0u32, 0usize, [0u16; 4]);
                    // action 0 = appui ; option 1 = ne pas composer les touches mortes
                    let err = UCKeyTranslate(layout, key, 0, state, kbd, 1, &mut dead, 4, &mut len, out.as_mut_ptr());
                    if err == 0 && len == 1 && out[0] == target && best.is_none_or(|b| (is_keypad(key), key) < b) {
                        best = Some((is_keypad(key), key));
                    }
                }
                if let Some((_, key)) = best {
                    found = Some((key, shift, option));
                    break 'search;
                }
            }
        }
        CFRelease(source);
        found
    }
}

/// Appuie sur la touche qui produit `c` (Maj / Option ajoutés si le caractère l'exige) ; sinon le caractère est tapé comme texte.
pub fn press_char(c: char) -> bool {
    match c {
        '\n' | '\r' => return send_scan(0x1C, false, false) & send_scan(0x1C, false, true),
        '\t' => return send_scan(0x0F, false, false) & send_scan(0x0F, false, true),
        c if c.is_control() => return true,
        _ => {}
    }
    let Some((key, shift, option)) = find_char(c) else { return send_char(c) };
    let extra = if shift { 0x0002_0000 } else { 0 } | if option { 0x0008_0000 } else { 0 };
    post_key(key, true, extra, None) & post_key(key, false, extra, None)
}

// ---------- anti-veille / arrêt d'urgence ----------
static AWAKE: Mutex<Option<Child>> = Mutex::new(None);

/// Empêche la veille tant qu'une action est armée (outil `caffeinate`, qui s'arrête de lui-même si AutoKey disparaît).
pub fn keep_awake(on: bool) {
    let Ok(mut slot) = AWAKE.lock() else { return };
    if let Some(mut child) = slot.take() {
        let _ = child.kill();
        let _ = child.wait();
    }
    if on {
        *slot = Command::new("caffeinate")
            .args(["-dis", "-w", &std::process::id().to_string()])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .ok();
    }
}

fn key_down(key: u16) -> bool {
    unsafe { CGEventSourceKeyState(COMBINED, key) }
}

/// Arrêt d'urgence : Ctrl + Option + Échap, même si la fenêtre est réduite.
pub fn panic_pressed() -> bool {
    (key_down(59) || key_down(62)) && (key_down(58) || key_down(61)) && key_down(53)
}

pub fn escape_down() -> bool {
    key_down(53)
}

pub fn left_button_down() -> bool {
    unsafe { CGEventSourceButtonState(COMBINED, 0) }
}

// ---------- souris ----------
pub fn cursor_pos() -> (i32, i32) {
    CGEventSource::new(CGEventSourceStateID::CombinedSessionState)
        .and_then(CGEvent::new)
        .map(|e| {
            let p = e.location();
            (p.x.round() as i32, p.y.round() as i32)
        })
        .unwrap_or((0, 0))
}

pub fn set_cursor_pos(x: i32, y: i32) {
    unsafe {
        CGWarpMouseCursorPosition(CGPoint::new(x as f64, y as f64));
    }
}

/// Clic gauche à l'écran en (x, y).
pub fn mouse_click(x: i32, y: i32) -> Result<(), String> {
    set_cursor_pos(x, y);
    sleep(Duration::from_millis(50));
    let (cx, cy) = cursor_pos();
    if (cx - x).abs() > 1 || (cy - y).abs() > 1 {
        return Err(i18n::t(Msg::ErrCursor).to_string());
    }
    let point = CGPoint::new(x as f64, y as f64);
    let send = |kind: CGEventType| -> bool {
        let Ok(source) = CGEventSource::new(CGEventSourceStateID::HIDSystemState) else { return false };
        match CGEvent::new_mouse_event(source, kind, point, CGMouseButton::Left) {
            Ok(event) => {
                event.post(CGEventTapLocation::HID);
                true
            }
            Err(()) => false,
        }
    };
    let down = send(CGEventType::LeftMouseDown);
    sleep(Duration::from_millis(30));
    let up = send(CGEventType::LeftMouseUp);
    sleep(Duration::from_millis(30));
    if down && up {
        Ok(())
    } else {
        Err(i18n::t(Msg::ErrClick).to_string())
    }
}

// ---------- fenêtres ----------
struct Win {
    id: isize,
    pid: i32,
    owner: String,
    title: String,
    layer: i64,
    rect: (i32, i32, i32, i32),
}

fn number(dict: &CFDictionary<CFString, CFType>, key: &str) -> Option<f64> {
    let v = dict.find(CFString::new(key))?;
    let n = v.downcast::<CFNumber>()?;
    n.to_f64()
}

fn text(dict: &CFDictionary<CFString, CFType>, key: &str) -> String {
    dict.find(CFString::new(key)).and_then(|v| v.downcast::<CFString>()).map(|s| s.to_string()).unwrap_or_default()
}

/// Fenêtres, de la plus haute à la plus basse. `all` inclut celles qui sont réduites ou sur un autre bureau.
fn windows(all: bool) -> Vec<Win> {
    let option = if all { kCGWindowListOptionAll } else { kCGWindowListOptionOnScreenOnly } | kCGWindowListExcludeDesktopElements;
    let Some(list) = copy_window_info(option, kCGNullWindowID) else { return Vec::new() };
    let mut out = Vec::new();
    for item in list.iter() {
        let dict: CFDictionary<CFString, CFType> = unsafe { CFDictionary::wrap_under_get_rule(*item as CFDictionaryRef) };
        let Some(id) = number(&dict, "kCGWindowNumber") else { continue };
        let rect = dict
            .find(CFString::new("kCGWindowBounds"))
            .and_then(|b| b.downcast::<CFDictionary>())
            .map(|b| {
                let b: CFDictionary<CFString, CFType> = unsafe { CFDictionary::wrap_under_get_rule(b.as_concrete_TypeRef()) };
                let (x, y) = (number(&b, "X").unwrap_or(0.0), number(&b, "Y").unwrap_or(0.0));
                let (w, h) = (number(&b, "Width").unwrap_or(0.0), number(&b, "Height").unwrap_or(0.0));
                (x as i32, y as i32, (x + w) as i32, (y + h) as i32)
            })
            .unwrap_or((0, 0, 0, 0));
        out.push(Win {
            id: id as isize,
            pid: number(&dict, "kCGWindowOwnerPID").unwrap_or(0.0) as i32,
            owner: text(&dict, "kCGWindowOwnerName"),
            title: text(&dict, "kCGWindowName"),
            layer: number(&dict, "kCGWindowLayer").unwrap_or(0.0) as i64,
            rect,
        });
    }
    out
}

fn info(h: isize) -> Option<Win> {
    windows(true).into_iter().find(|w| w.id == h)
}

pub fn win_title(h: isize) -> String {
    info(h).map(|w| w.title).unwrap_or_default()
}

/// Sous macOS, la « classe » est le nom de l'application propriétaire.
pub fn win_class(h: isize) -> String {
    info(h).map(|w| w.owner).unwrap_or_default()
}

pub fn win_exe(h: isize) -> String {
    info(h).map(|w| w.owner.to_lowercase()).unwrap_or_default()
}

pub fn win_pid(h: isize) -> u32 {
    info(h).map_or(0, |w| w.pid as u32)
}

pub fn win_rect(h: isize) -> (i32, i32, i32, i32) {
    info(h).map_or((0, 0, 0, 0), |w| w.rect)
}

pub fn is_window(h: isize) -> bool {
    h != 0 && info(h).is_some()
}

/// Fenêtre normale la plus haute (celle de l'application au premier plan).
pub fn foreground() -> isize {
    windows(false).into_iter().find(|w| w.layer == 0).map_or(0, |w| w.id)
}

/// Fenêtre de premier niveau visible située sous un point de l'écran.
pub fn root_at(x: i32, y: i32) -> isize {
    windows(false)
        .into_iter()
        .find(|w| w.layer == 0 && x >= w.rect.0 && x < w.rect.2 && y >= w.rect.1 && y < w.rect.3)
        .map_or(0, |w| w.id)
}

/// Retrouve la fenêtre cible : même identifiant si elle existe encore, sinon même application.
pub fn find_window(t: &Target) -> Option<isize> {
    let all: Vec<Win> = windows(true).into_iter().filter(|w| w.layer == 0 && w.rect.2 > w.rect.0).collect();
    if let Some(w) = all.iter().find(|w| w.id == t.hwnd && w.owner == t.cls) {
        return Some(w.id);
    }
    let same: Vec<&Win> = all.iter().filter(|w| w.owner == t.cls && w.owner.to_lowercase() == t.exe).collect();
    same.iter().find(|w| w.title == t.title).or(same.first()).map(|w| w.id)
}

/// Amène l'application de la fenêtre au premier plan. Vrai si réussi ; `stop` permet d'interrompre l'attente.
pub fn focus_window(h: isize, stop: &dyn Fn() -> bool) -> bool {
    let Some(win) = info(h) else { return false };
    if foreground() == h {
        return true;
    }
    // `open -a` active l'application (et rouvre sa fenêtre si elle est réduite) sans demander d'autorisation supplémentaire
    let _ = Command::new("open").args(["-a", &win.owner]).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status();
    for _ in 0..40 {
        sleep(Duration::from_millis(50));
        if stop() {
            return false;
        }
        if windows(false).into_iter().find(|w| w.layer == 0).is_some_and(|w| w.pid == win.pid) {
            return true;
        }
    }
    false
}

// ---------- ouvrir un lien, instance unique, identité ----------
/// Ouvre une adresse web dans le navigateur par défaut.
pub fn open_url(url: &str) {
    let _ = Command::new("open").arg(url).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn();
}

/// Vrai si une autre instance d'AutoKey tourne déjà. (Deux instances armées taperaient deux fois.)
pub fn another_instance_running(_window_title: &str) -> bool {
    static LOCK: OnceLock<std::fs::File> = OnceLock::new();
    let path: PathBuf = std::env::temp_dir().join("autokey.lock");
    let Ok(file) = std::fs::OpenOptions::new().create(true).truncate(false).write(true).open(path) else { return false };
    if file.try_lock().is_ok() {
        let _ = LOCK.set(file);
        return false;
    }
    true
}

pub fn own_pid() -> u32 {
    std::process::id()
}

/// Disposition et variante du clavier actif, au même format que sous Linux (« fr », « us » / « dvorak »…).
pub fn xkb_layout() -> Option<(String, String)> {
    let out = Command::new("defaults").args(["read", "com.apple.HIToolbox", "AppleCurrentKeyboardLayoutInputSourceID"]).output().ok()?;
    let id = String::from_utf8(out.stdout).ok()?;
    let name = id.trim().rsplit('.').next()?.to_lowercase();
    let (layout, variant) = match name.as_str() {
        "us" | "abc" | "usextended" => ("us", ""),
        "french" | "french-pc" => ("fr", ""),
        "belgian" => ("be", ""),
        "british" | "british-pc" => ("gb", ""),
        "german" | "austrian" => ("de", ""),
        "swissfrench" | "swissgerman" => ("ch", ""),
        "spanish" | "spanish-iso" => ("es", ""),
        "italian" | "italian-pro" => ("it", ""),
        "brazilian" | "brazilian-pro" => ("br", ""),
        "russian" | "russianwin" => ("ru", ""),
        "arabic" | "arabic-azerty" | "arabic-qwerty" => ("ara", ""),
        "dvorak" => ("us", "dvorak"),
        "colemak" => ("us", "colemak"),
        _ => return None,
    };
    Some((layout.to_string(), variant.to_string()))
}

// ---------- bannière « clique dans la zone à cibler » ----------
/// Sous macOS, aucune bannière : le message d'état de la fenêtre AutoKey sert d'indication.
pub struct Banner;

impl Banner {
    pub fn show(_text: &str) -> Banner {
        Banner
    }
}
