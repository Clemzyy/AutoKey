//! Moteur Linux (X11) : envoi de touches par XTest, fenêtres par EWMH. Aucune interface ici.
//!
//! Sous Wayland, le système interdit à un programme d'envoyer des touches à une autre fenêtre ou de les lister :
//! seules les fenêtres XWayland sont alors visibles (voir `session_warning`).
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::{sleep, JoinHandle};
use std::time::{Duration, Instant};

use x11rb::connection::Connection;
use x11rb::protocol::xproto::{
    self, AtomEnum, ClientMessageEvent, ConfigureWindowAux, ConnectionExt as _, CreateGCAux, CreateWindowAux, EventMask, InputFocus, MapState,
    StackMode, Window, WindowClass,
};
use x11rb::protocol::xtest::ConnectionExt as _;
use x11rb::protocol::Event;
use x11rb::rust_connection::RustConnection;
use xkeysym::Keysym;

use super::Target;
use crate::i18n::{self, Msg};
use crate::keys::Modk;

struct Atoms {
    client_list: u32,
    stacking: u32,
    active: u32,
    wm_name: u32,
    utf8: u32,
    pid: u32,
    state: u32,
    hidden: u32,
    wm_class: u32,
    xkb_rules: u32,
}

struct X {
    conn: RustConnection,
    root: Window,
    atoms: Atoms,
    min_key: u8,
    max_key: u8,
}

/// Connexion unique au serveur X (absente si aucune session X11 / XWayland n'est joignable).
fn x() -> Option<&'static X> {
    static X: OnceLock<Option<X>> = OnceLock::new();
    X.get_or_init(|| {
        let (conn, screen) = RustConnection::connect(None).ok()?;
        let root = conn.setup().roots.get(screen)?.root;
        let atom = |name: &[u8]| -> Option<u32> { Some(conn.intern_atom(false, name).ok()?.reply().ok()?.atom) };
        let atoms = Atoms {
            client_list: atom(b"_NET_CLIENT_LIST")?,
            stacking: atom(b"_NET_CLIENT_LIST_STACKING")?,
            active: atom(b"_NET_ACTIVE_WINDOW")?,
            wm_name: atom(b"_NET_WM_NAME")?,
            utf8: atom(b"UTF8_STRING")?,
            pid: atom(b"_NET_WM_PID")?,
            state: atom(b"_NET_WM_STATE")?,
            hidden: atom(b"_NET_WM_STATE_HIDDEN")?,
            wm_class: atom(b"WM_CLASS")?,
            xkb_rules: atom(b"_XKB_RULES_NAMES")?,
        };
        let (min_key, max_key) = (conn.setup().min_keycode, conn.setup().max_keycode);
        Some(X { conn, root, atoms, min_key, max_key })
    })
    .as_ref()
}

/// Message à afficher si le système ne permet pas d'agir sur les autres fenêtres (Wayland).
pub fn session_warning() -> Option<Msg> {
    let wayland = std::env::var("XDG_SESSION_TYPE").is_ok_and(|v| v.eq_ignore_ascii_case("wayland"));
    (wayland || x().is_none()).then_some(Msg::WaylandWarning)
}

// ---------- envoi de touches ----------
const KEY_PRESS: u8 = 2;
const KEY_RELEASE: u8 = 3;
const BUTTON_PRESS: u8 = 4;
const BUTTON_RELEASE: u8 = 5;
const MOTION: u8 = 6;

/// Événement de test XTest ; vrai si la requête est partie.
fn fake(x: &X, kind: u8, detail: u8, rx: i16, ry: i16) -> bool {
    x.conn.xtest_fake_input(kind, detail, 0, x.root, rx, ry, 0).is_ok() && x.conn.flush().is_ok()
}

/// Symboles X11 de la touche désignée par un code de balayage Windows (jeu 1). Les numéros de touches X11 varient selon
/// le serveur (matériel, session distante…) : on retrouve donc la touche par son symbole dans la disposition active.
fn keysyms(scan: u16, ext: bool) -> Vec<u32> {
    match (scan, ext) {
        (0x01, false) => vec![0xff1b],                                    // Échap
        (0x0E, false) => vec![0xff08],                                    // Retour arrière
        (0x0F, false) => vec![0xff09],                                    // Tabulation
        (0x1C, false) => vec![0xff0d],                                    // Entrée
        (0x1D, false) => vec![0xffe3],                                    // Ctrl gauche
        (0x2A, false) => vec![0xffe1],                                    // Maj gauche
        (0x36, false) => vec![0xffe2],                                    // Maj droite
        (0x38, false) => vec![0xffe9],                                    // Alt
        (0x38, true) => vec![0xfe03, 0xffea, 0xff7e],                     // Alt Gr
        (0x39, false) => vec![0x20],                                      // Espace
        (0x3A, false) => vec![0xffe5],                                    // Verr. Maj
        (n @ 0x3B..=0x44, false) => vec![0xffbe + (n - 0x3B) as u32],     // F1 à F10
        (0x57, false) => vec![0xffc8],                                    // F11
        (0x58, false) => vec![0xffc9],                                    // F12
        (0x47, true) => vec![0xff50],                                     // Début
        (0x48, true) => vec![0xff52],                                     // Haut
        (0x49, true) => vec![0xff55],                                     // Page précédente
        (0x4B, true) => vec![0xff51],                                     // Gauche
        (0x4D, true) => vec![0xff53],                                     // Droite
        (0x4F, true) => vec![0xff57],                                     // Fin
        (0x50, true) => vec![0xff54],                                     // Bas
        (0x51, true) => vec![0xff56],                                     // Page suivante
        (0x52, true) => vec![0xff63],                                     // Insertion
        (0x53, true) => vec![0xffff],                                     // Suppression
        _ => Vec::new(),
    }
}

/// Appuie (up = false) ou relâche (up = true) une touche physique, par code de balayage.
pub fn send_scan(scan: u16, ext: bool, up: bool) -> bool {
    let Some(x) = x() else { return false };
    let Some(km) = keymap(x) else { return false };
    let Some(code) = keysyms(scan, ext).iter().find_map(|s| km.codes_for(*s).first().copied()) else { return false };
    fake(x, if up { KEY_RELEASE } else { KEY_PRESS }, code, 0, 0)
}

struct Keymap {
    first: u8,
    per: usize,
    syms: Vec<u32>,
}

impl Keymap {
    /// Touches dont le symbole de base est `sym`.
    fn codes_for(&self, sym: u32) -> Vec<u8> {
        self.syms.chunks(self.per.max(1)).enumerate().filter(|(_, e)| e.first() == Some(&sym)).map(|(i, _)| self.first.wrapping_add(i as u8)).collect()
    }
}

/// Disposition courante, relue au plus une fois par seconde (elle change rarement).
fn keymap(x: &X) -> Option<Arc<Keymap>> {
    static CACHE: Mutex<Option<(Instant, Arc<Keymap>)>> = Mutex::new(None);
    let mut slot = CACHE.lock().ok()?;
    if let Some((at, km)) = slot.as_ref() {
        if at.elapsed() < Duration::from_secs(1) {
            return Some(km.clone());
        }
    }
    let count = x.max_key - x.min_key + 1;
    let r = x.conn.get_keyboard_mapping(x.min_key, count).ok()?.reply().ok()?;
    let km = Arc::new(Keymap { first: x.min_key, per: r.keysyms_per_keycode as usize, syms: r.keysyms });
    *slot = Some((Instant::now(), km.clone()));
    Some(km)
}

/// (touche, Maj, Alt Gr) qui produit `c` avec la disposition active ; `None` si elle ne le contient pas.
fn find_char(km: &Keymap, c: char) -> Option<(u8, bool, bool)> {
    let mut best: Option<((bool, usize, u8), (u8, bool, bool))> = None;
    for (i, entry) in km.syms.chunks(km.per.max(1)).enumerate() {
        let code = km.first.wrapping_add(i as u8);
        // dans le protocole X11 : 0 = base, 1 = Maj, 4 = Alt Gr, 5 = Alt Gr + Maj (2 et 3 sont le second groupe)
        for (idx, shift, altgr) in [(0, false, false), (1, true, false), (4, false, true), (5, true, true)] {
            if idx < entry.len() && Keysym::new(entry[idx]).key_char() == Some(c) {
                // pavé numérique (KP_*) : à éviter, son effet dépend de Verr. Num
                let keypad = (0xff80..=0xffbd).contains(&entry[idx]);
                let rank = (keypad, shift as usize + 2 * altgr as usize, code);
                if best.as_ref().is_none_or(|(r, _)| rank < *r) {
                    best = Some((rank, (code, shift, altgr)));
                }
                break;
            }
        }
    }
    best.map(|(_, plan)| plan)
}

/// Tape un caractère absent de la disposition : une touche libre reçoit temporairement son symbole.
fn type_unicode(x: &X, km: &Keymap, c: char) -> bool {
    let Some(spare) = km.syms.chunks(km.per.max(1)).position(|e| e.iter().all(|s| *s == 0)) else { return false };
    let code = km.first + spare as u8;
    let sym = Keysym::from_char(c).raw();
    let per = km.per.max(1);
    let set = |syms: &[u32]| x.conn.change_keyboard_mapping(1, code, per as u8, syms).is_ok() && x.conn.flush().is_ok();
    if !set(&vec![sym; per]) {
        return false;
    }
    sleep(Duration::from_millis(12)); // le temps que les applications relisent la disposition
    let ok = fake(x, KEY_PRESS, code, 0, 0) & fake(x, KEY_RELEASE, code, 0, 0);
    sleep(Duration::from_millis(12));
    set(&vec![0; per]);
    ok
}

/// Appuie sur la touche qui produit `c` (Maj / Alt Gr ajoutés si le caractère l'exige) ; si la disposition active ne
/// le contient pas, il est tapé par une touche temporaire.
pub fn press_char(c: char) -> bool {
    let Some(x) = x() else { return false };
    match c {
        '\n' | '\r' => return send_scan(0x1C, false, false) & send_scan(0x1C, false, true),
        '\t' => return send_scan(0x0F, false, false) & send_scan(0x0F, false, true),
        c if c.is_control() => return true,
        _ => {}
    }
    let Some(km) = keymap(x) else { return false };
    let Some((code, shift, altgr)) = find_char(&km, c) else { return type_unicode(x, &km, c) };
    let mut held: Vec<(u16, bool)> = Vec::new();
    if shift {
        held.push(Modk::LShift.scan());
    }
    if altgr {
        held.push(Modk::AltGr.scan());
    }
    let mut ok = true;
    for (s, e) in &held {
        ok &= send_scan(*s, *e, false);
    }
    ok &= fake(x, KEY_PRESS, code, 0, 0);
    ok &= fake(x, KEY_RELEASE, code, 0, 0);
    for (s, e) in held.iter().rev() {
        send_scan(*s, *e, true);
    }
    ok
}

/// Sous X11, le clavier a une seule disposition : taper un caractère ou appuyer sur sa touche revient au même.
pub fn send_char(c: char) -> bool {
    press_char(c)
}

// ---------- anti-veille / arrêt d'urgence ----------
static INHIBIT: Mutex<Option<Child>> = Mutex::new(None);

/// Empêche la veille tant qu'une action est armée (via `systemd-inhibit` ; sans effet s'il est absent).
/// Le processus se termine de lui-même si AutoKey disparaît.
pub fn keep_awake(on: bool) {
    let Ok(mut slot) = INHIBIT.lock() else { return };
    if let Some(mut child) = slot.take() {
        let _ = child.kill();
        let _ = child.wait();
    }
    if on {
        let watch = format!("while kill -0 {} 2>/dev/null; do sleep 2; done", std::process::id());
        *slot = Command::new("systemd-inhibit")
            .args(["--what=idle:sleep", "--who=AutoKey", "--why=Action armee", "--mode=block", "sh", "-c", &watch])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .ok();
    }
}

fn pressed_keys(x: &X) -> Option<[u8; 32]> {
    x.conn.query_keymap().ok()?.reply().ok().map(|r| r.keys)
}

fn is_down(keys: &[u8; 32], code: u8) -> bool {
    keys[(code / 8) as usize] & (1 << (code % 8)) != 0
}

/// Vrai si l'une des touches portant l'un de ces symboles est enfoncée.
fn any_down(x: &X, keys: &[u8; 32], syms: &[u32]) -> bool {
    keymap(x).is_some_and(|km| syms.iter().flat_map(|s| km.codes_for(*s)).any(|c| is_down(keys, c)))
}

/// Arrêt d'urgence : Ctrl + Alt + Échap, même si la fenêtre est réduite.
pub fn panic_pressed() -> bool {
    let Some(x) = x() else { return false };
    let Some(keys) = pressed_keys(x) else { return false };
    any_down(x, &keys, &[0xffe3, 0xffe4]) && any_down(x, &keys, &[0xffe9, 0xffea]) && any_down(x, &keys, &[0xff1b])
}

pub fn escape_down() -> bool {
    let Some(x) = x() else { return false };
    pressed_keys(x).is_some_and(|keys| any_down(x, &keys, &[0xff1b]))
}

fn pointer(x: &X) -> Option<xproto::QueryPointerReply> {
    x.conn.query_pointer(x.root).ok()?.reply().ok()
}

/// Bouton principal de la souris (X11 applique déjà l'inversion des boutons choisie par l'utilisateur).
pub fn left_button_down() -> bool {
    x().and_then(pointer).is_some_and(|p| u16::from(p.mask) & u16::from(xproto::KeyButMask::BUTTON1) != 0)
}

pub fn cursor_pos() -> (i32, i32) {
    x().and_then(pointer).map_or((0, 0), |p| (p.root_x as i32, p.root_y as i32))
}

pub fn set_cursor_pos(px: i32, py: i32) {
    if let Some(x) = x() {
        fake(x, MOTION, 0, px as i16, py as i16);
    }
}

// ---------- informations de fenêtre ----------
fn prop(x: &X, win: u32, atom: u32, ty: impl Into<u32>) -> Option<xproto::GetPropertyReply> {
    x.conn.get_property(false, win, atom, ty, 0, 4096).ok()?.reply().ok().filter(|r| r.value_len > 0)
}

fn prop32(x: &X, win: u32, atom: u32, ty: impl Into<u32>) -> Vec<u32> {
    prop(x, win, atom, ty).and_then(|r| r.value32().map(|i| i.collect())).unwrap_or_default()
}

pub fn win_title(h: isize) -> String {
    let Some(x) = x() else { return String::new() };
    let r = prop(x, h as u32, x.atoms.wm_name, x.atoms.utf8).or_else(|| prop(x, h as u32, u32::from(AtomEnum::WM_NAME), AtomEnum::ANY));
    r.map(|r| String::from_utf8_lossy(&r.value).into_owned()).unwrap_or_default()
}

/// Classe de la fenêtre (WM_CLASS : « instance », « classe »).
pub fn win_class(h: isize) -> String {
    let Some(x) = x() else { return String::new() };
    prop(x, h as u32, x.atoms.wm_class, AtomEnum::ANY)
        .map(|r| {
            let parts: Vec<&[u8]> = r.value.split(|b| *b == 0).filter(|p| !p.is_empty()).collect();
            String::from_utf8_lossy(parts.get(1).or(parts.first()).copied().unwrap_or(&[])).into_owned()
        })
        .unwrap_or_default()
}

pub fn win_pid(h: isize) -> u32 {
    let Some(x) = x() else { return 0 };
    prop32(x, h as u32, x.atoms.pid, AtomEnum::CARDINAL).first().copied().unwrap_or(0)
}

pub fn win_exe(h: isize) -> String {
    let pid = win_pid(h);
    if pid == 0 {
        return String::new();
    }
    let from_exe = std::fs::read_link(format!("/proc/{pid}/exe")).ok().and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()));
    let name = from_exe.or_else(|| std::fs::read_to_string(format!("/proc/{pid}/comm")).ok().map(|s| s.trim().to_string()));
    name.unwrap_or_default().to_lowercase()
}

pub fn win_rect(h: isize) -> (i32, i32, i32, i32) {
    let Some(x) = x() else { return (0, 0, 0, 0) };
    let win = h as u32;
    let geometry = x.conn.get_geometry(win).ok().and_then(|c| c.reply().ok());
    let origin = x.conn.translate_coordinates(win, x.root, 0, 0).ok().and_then(|c| c.reply().ok());
    match (geometry, origin) {
        (Some(g), Some(o)) => (o.dst_x as i32, o.dst_y as i32, o.dst_x as i32 + g.width as i32, o.dst_y as i32 + g.height as i32),
        _ => (0, 0, 0, 0),
    }
}

pub fn foreground() -> isize {
    let Some(x) = x() else { return 0 };
    prop32(x, x.root, x.atoms.active, AtomEnum::WINDOW).first().copied().unwrap_or(0) as isize
}

pub fn is_window(h: isize) -> bool {
    h != 0 && x().is_some_and(|x| x.conn.get_geometry(h as u32).ok().and_then(|c| c.reply().ok()).is_some())
}

fn is_hidden(x: &X, win: u32) -> bool {
    prop32(x, win, x.atoms.state, AtomEnum::ATOM).contains(&x.atoms.hidden)
}

fn is_viewable(x: &X, win: u32) -> bool {
    x.conn.get_window_attributes(win).ok().and_then(|c| c.reply().ok()).is_some_and(|a| a.map_state == MapState::VIEWABLE)
}

/// Fenêtres de premier niveau, de la plus basse à la plus haute.
fn stacked(x: &X) -> Vec<u32> {
    let list = prop32(x, x.root, x.atoms.stacking, AtomEnum::WINDOW);
    if list.is_empty() {
        prop32(x, x.root, x.atoms.client_list, AtomEnum::WINDOW)
    } else {
        list
    }
}

/// Fenêtre de premier niveau visible située sous un point de l'écran.
pub fn root_at(px: i32, py: i32) -> isize {
    let Some(x) = x() else { return 0 };
    for win in stacked(x).into_iter().rev() {
        if is_hidden(x, win) || !is_viewable(x, win) {
            continue;
        }
        let (l, t, r, b) = win_rect(win as isize);
        if px >= l && px < r && py >= t && py < b {
            return win as isize;
        }
    }
    0
}

// ---------- souris ----------
/// Clic gauche à l'écran en (x, y).
pub fn mouse_click(px: i32, py: i32) -> Result<(), String> {
    let Some(x) = x() else { return Err(i18n::t(Msg::ErrClick).to_string()) };
    set_cursor_pos(px, py);
    sleep(Duration::from_millis(50));
    if cursor_pos() != (px, py) {
        return Err(i18n::t(Msg::ErrCursor).to_string());
    }
    let down_ok = fake(x, BUTTON_PRESS, 1, 0, 0);
    sleep(Duration::from_millis(30));
    let up_ok = fake(x, BUTTON_RELEASE, 1, 0, 0);
    sleep(Duration::from_millis(30));
    if down_ok && up_ok {
        Ok(())
    } else {
        Err(i18n::t(Msg::ErrClick).to_string())
    }
}

// ---------- retrouver, focaliser ----------
/// Retrouve la fenêtre cible : même identifiant si elle existe encore, sinon même programme + même classe.
pub fn find_window(t: &Target) -> Option<isize> {
    if is_window(t.hwnd) && win_class(t.hwnd) == t.cls && win_exe(t.hwnd) == t.exe {
        return Some(t.hwnd);
    }
    let x = x()?;
    let found: Vec<isize> = prop32(x, x.root, x.atoms.client_list, AtomEnum::WINDOW)
        .into_iter()
        .map(|w| w as isize)
        .filter(|h| win_class(*h) == t.cls && win_exe(*h) == t.exe)
        .collect();
    found.iter().copied().find(|h| win_title(*h) == t.title).or_else(|| found.first().copied())
}

/// Amène la fenêtre au premier plan (la restaure si réduite). Vrai si réussi ; `stop` permet d'interrompre l'attente.
pub fn focus_window(h: isize, stop: &dyn Fn() -> bool) -> bool {
    let Some(x) = x() else { return false };
    let win = h as u32;
    for attempt in 0..2 {
        if foreground() == h {
            return true;
        }
        // demande « EWMH » ; la source 2 (outil de pagination) contourne la protection contre le vol de focus
        let event = ClientMessageEvent::new(32, win, x.atoms.active, [2u32, 0, 0, 0, 0]);
        let _ = x.conn.send_event(false, x.root, EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY, event);
        if attempt > 0 {
            let _ = x.conn.map_window(win);
            let _ = x.conn.configure_window(win, &ConfigureWindowAux::new().stack_mode(StackMode::ABOVE));
            let _ = x.conn.set_input_focus(InputFocus::POINTER_ROOT, win, 0u32);
        }
        let _ = x.conn.flush();
        for _ in 0..20 {
            sleep(Duration::from_millis(50));
            if stop() {
                return false;
            }
            if foreground() == h {
                return true;
            }
        }
    }
    false
}

// ---------- ouvrir un lien ----------
/// Ouvre une adresse web dans le navigateur par défaut.
pub fn open_url(url: &str) {
    let _ = Command::new("xdg-open").arg(url).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn();
}

// ---------- disposition du clavier ----------
/// Disposition et variante XKB déclarées par la session (« fr », « us,ru » / « dvorak »…).
pub fn xkb_layout() -> Option<(String, String)> {
    let x = x()?;
    let r = prop(x, x.root, x.atoms.xkb_rules, AtomEnum::STRING)?;
    let parts: Vec<String> = r.value.split(|b| *b == 0).map(|p| String::from_utf8_lossy(p).into_owned()).collect();
    // règles, modèle, disposition, variante, options
    Some((parts.get(2)?.clone(), parts.get(3).cloned().unwrap_or_default()))
}

// ---------- instance unique ----------
/// Vrai si une autre instance d'AutoKey tourne déjà ; dans ce cas sa fenêtre est ramenée au premier plan.
/// (Deux instances armées taperaient deux fois et s'écraseraient les réglages.)
pub fn another_instance_running(window_title: &str) -> bool {
    static LOCK: OnceLock<std::fs::File> = OnceLock::new();
    let dir = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    let Ok(file) = std::fs::OpenOptions::new().create(true).truncate(false).write(true).open(dir.join("autokey.lock")) else {
        return false;
    };
    if file.try_lock().is_ok() {
        let _ = LOCK.set(file); // verrou gardé jusqu'à la fin du processus
        return false;
    }
    if let Some(x) = x() {
        let own = own_pid();
        let other = prop32(x, x.root, x.atoms.client_list, AtomEnum::WINDOW)
            .into_iter()
            .map(|w| w as isize)
            .find(|h| win_pid(*h) != own && win_title(*h) == window_title);
        if let Some(h) = other {
            focus_window(h, &|| false);
        }
    }
    true
}

// ---------- identité du processus ----------
pub fn own_pid() -> u32 {
    std::process::id()
}

// ---------- bannière « clique dans la zone à cibler » ----------
/// Petite fenêtre toujours visible, sans prise de focus, affichée pendant le repérage de la zone.
pub struct Banner {
    close: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Banner {
    pub fn show(text: &str) -> Banner {
        let close = Arc::new(AtomicBool::new(false));
        let (c2, text) = (close.clone(), text.to_string());
        let thread = std::thread::spawn(move || {
            banner_thread(&text, &c2);
        });
        Banner { close, thread: Some(thread) }
    }
}

impl Drop for Banner {
    fn drop(&mut self) {
        self.close.store(true, Ordering::SeqCst);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// Le serveur X ne dessine que les polices « bitmap » : au-delà du latin, du grec et du cyrillique, on affiche l'anglais.
fn banner_text(text: &str) -> String {
    if text.chars().all(|c| (c as u32) < 0x530) {
        text.to_string()
    } else {
        "AutoKey: click in the target input area (Esc = cancel)".to_string()
    }
}

fn banner_thread(text: &str, close: &AtomicBool) -> Option<()> {
    let (conn, screen) = RustConnection::connect(None).ok()?;
    let scr = conn.setup().roots.get(screen)?.clone();
    let (bg, fg) = (0x1b2a3a_u32, 0xffffff_u32);
    // police et largeur du texte d'abord : la fenêtre s'adapte au message
    let font = conn.generate_id().ok()?;
    let names = ["-*-*-bold-r-normal--20-*-*-*-*-*-iso10646-1", "-*-*-medium-r-normal--20-*-*-*-*-*-iso10646-1", "fixed"];
    let opened = names.iter().any(|n| conn.open_font(font, n.as_bytes()).ok().is_some_and(|c| c.check().is_ok()));
    let chars: Vec<xproto::Char2b> = banner_text(text).chars().filter(|c| (*c as u32) < 0x10000).map(|c| xproto::Char2b { byte1: (c as u32 >> 8) as u8, byte2: c as u8 }).collect();
    let width = if opened { conn.query_text_extents(font, &chars).ok()?.reply().ok()?.overall_width } else { 0 };
    let w = ((width + 48).max(680) as u16).min(scr.width_in_pixels.saturating_sub(40));
    let h = 60u16;
    let win = conn.generate_id().ok()?;
    let aux = CreateWindowAux::new().override_redirect(1).background_pixel(bg).border_pixel(fg).event_mask(EventMask::EXPOSURE);
    conn.create_window(scr.root_depth, win, scr.root, 20, 20, w, h, 2, WindowClass::INPUT_OUTPUT, scr.root_visual, &aux).ok()?;
    let gc = conn.generate_id().ok()?;
    let mut gc_aux = CreateGCAux::new().foreground(fg).background(bg);
    if opened {
        gc_aux = gc_aux.font(font);
    }
    conn.create_gc(gc, win, &gc_aux).ok()?;
    let draw = || {
        let x = ((w as i32 - width) / 2).max(8) as i16;
        let _ = conn.image_text16(win, gc, x, (h / 2 + 7) as i16, &chars);
        let _ = conn.flush();
    };
    conn.map_window(win).ok()?;
    conn.flush().ok()?;
    while !close.load(Ordering::SeqCst) {
        while let Ok(Some(event)) = conn.poll_for_event() {
            if matches!(event, Event::Expose(_)) {
                draw();
            }
        }
        sleep(Duration::from_millis(20));
    }
    let _ = conn.destroy_window(win);
    let _ = conn.flush();
    Some(())
}
