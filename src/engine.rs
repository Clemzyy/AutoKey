//! Moteur Windows : envoi de touches, fenêtres, souris, anti-veille. Aucune interface ici.
use std::mem::size_of;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{sleep, JoinHandle};
use std::time::Duration;

use crate::i18n::{self, Msg};
use crate::keys::Modk;
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HWND, LPARAM, POINT, RECT};
use windows_sys::Win32::System::Power::{
    SetThreadExecutionState, ES_CONTINUOUS, ES_DISPLAY_REQUIRED, ES_SYSTEM_REQUIRED,
};
use windows_sys::Win32::Graphics::Gdi::{CreateFontW, DeleteObject};
use windows_sys::Win32::System::Threading::{
    AttachThreadInput, CreateMutexW, GetCurrentProcessId, GetCurrentThreadId, OpenProcess, QueryFullProcessImageNameW,
};
use windows_sys::Win32::UI::HiDpi::GetDpiForSystem;
use windows_sys::Win32::UI::Shell::ShellExecuteW;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    keybd_event, GetAsyncKeyState, GetKeyboardLayout, MapVirtualKeyExW, SendInput, VkKeyScanExW, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT,
    KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE, KEYEVENTF_UNICODE, MOUSEEVENTF_LEFTDOWN,
    MOUSEEVENTF_LEFTUP, MOUSEINPUT, VK_CONTROL, VK_ESCAPE, VK_LBUTTON, VK_MENU, VK_RBUTTON,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, EnumWindows, GetAncestor, GetClassNameW, GetCursorPos,
    GetForegroundWindow, GetWindowRect, GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindow,
    GetSystemMetrics, IsWindowVisible, SetCursorPos, SetForegroundWindow, ShowWindowAsync, WindowFromPoint, GA_ROOT, SW_RESTORE,
    CreateWindowExW, DestroyWindow, FindWindowW, DispatchMessageW, PeekMessageW, SendMessageW, TranslateMessage, MSG,
    PM_REMOVE, SM_SWAPBUTTON, WM_SETFONT, WS_BORDER, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST, WS_POPUP, WS_VISIBLE,
};

/// Fenêtre + position de la zone de saisie visée (la fenêtre est retrouvée même après un redémarrage).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Target {
    pub hwnd: isize, // 0 = inconnu (non sauvegardé)
    pub exe: String,
    pub cls: String,
    pub title: String,
    pub dx: i32,
    pub dy: i32,
    pub w: i32,
    pub h: i32,
    pub fx: f64,
    pub fy: f64,
}

fn hwnd(h: isize) -> HWND {
    h as HWND
}

// ---------- envoi de touches ----------
/// Vrai si Windows a accepté l'événement (faux, par ex., si la fenêtre active tourne en administrateur).
fn send_input(input: INPUT) -> bool {
    unsafe { SendInput(1, &input, size_of::<INPUT>() as i32) == 1 }
}

fn key_input(w_vk: u16, scan: u16, flags: u32) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: w_vk, wScan: scan, dwFlags: flags, time: 0, dwExtraInfo: 0 } },
    }
}

/// Appuie (up = false) ou relâche (up = true) une touche physique, par code de balayage.
pub fn send_scan(scan: u16, ext: bool, up: bool) -> bool {
    let mut flags = KEYEVENTF_SCANCODE;
    if ext {
        flags |= KEYEVENTF_EXTENDEDKEY;
    }
    if up {
        flags |= KEYEVENTF_KEYUP;
    }
    send_input(key_input(0, scan, flags))
}

/// Tape un caractère quelconque (accents, symboles…), indépendamment de la disposition du clavier.
pub fn send_char(c: char) -> bool {
    let mut buf = [0u16; 2];
    let mut ok = true;
    for unit in c.encode_utf16(&mut buf) {
        ok &= send_input(key_input(0, *unit, KEYEVENTF_UNICODE));
        ok &= send_input(key_input(0, *unit, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP));
    }
    ok
}

/// Appuie sur la touche qui produit `c` avec la disposition de clavier **de la fenêtre qui reçoit les touches**
/// (maj/AltGr ajoutés si le caractère l'exige). Si la disposition active ne contient pas ce caractère, il est tapé
/// comme texte (Unicode). Renvoie faux si Windows a refusé un événement.
pub fn press_char(c: char) -> bool {
    let mut buf = [0u16; 2];
    let units = c.encode_utf16(&mut buf);
    if units.len() != 1 {
        return send_char(c);
    }
    unsafe {
        let fg = GetForegroundWindow();
        let thread = if fg.is_null() { 0 } else { GetWindowThreadProcessId(fg, std::ptr::null_mut()) };
        let hkl = GetKeyboardLayout(thread);
        let r = VkKeyScanExW(units[0], hkl);
        if r == -1 {
            return send_char(c);
        }
        let (vk, state) = ((r & 0xFF) as u32, ((r >> 8) & 0xFF) as u8);
        let sc = MapVirtualKeyExW(vk, 4, hkl); // MAPVK_VK_TO_VSC_EX
        if sc == 0 {
            return send_char(c);
        }
        let (scan, ext) = ((sc & 0xFF) as u16, sc >> 8 == 0xE0 || sc >> 8 == 0xE1);
        // modificateurs exigés par le caractère : bit 0 = Maj, bit 1 = Ctrl, bit 2 = Alt (Ctrl + Alt = AltGr)
        let mut held: Vec<(u16, bool)> = Vec::new();
        if state & 1 != 0 {
            held.push(Modk::LShift.scan());
        }
        if state & 6 == 6 {
            held.push(Modk::AltGr.scan());
        } else {
            if state & 2 != 0 {
                held.push(Modk::Ctrl.scan());
            }
            if state & 4 != 0 {
                held.push(Modk::Alt.scan());
            }
        }
        let mut ok = true;
        for (s, e) in &held {
            ok &= send_scan(*s, *e, false);
        }
        ok &= send_scan(scan, ext, false);
        ok &= send_scan(scan, ext, true);
        for (s, e) in held.iter().rev() {
            send_scan(*s, *e, true);
        }
        ok
    }
}

// ---------- anti-veille / arrêt d'urgence ----------
/// Empêche la veille (et l'extinction de l'écran) tant qu'une action est armée.
/// À appeler depuis le même thread pour l'activer et le désactiver.
pub fn keep_awake(on: bool) {
    let flags = if on { ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED } else { ES_CONTINUOUS };
    unsafe {
        SetThreadExecutionState(flags);
    }
}

fn down(vk: u16) -> bool {
    unsafe { GetAsyncKeyState(vk as i32) < 0 }
}

/// Arrêt d'urgence : Ctrl + Alt + Échap, même si la fenêtre est réduite.
pub fn panic_pressed() -> bool {
    down(VK_CONTROL) && down(VK_MENU) && down(VK_ESCAPE)
}

pub fn escape_down() -> bool {
    down(VK_ESCAPE)
}

/// Bouton principal de la souris (le physique de droite si l'utilisateur a inversé les boutons).
pub fn left_button_down() -> bool {
    let swapped = unsafe { GetSystemMetrics(SM_SWAPBUTTON) != 0 };
    down(if swapped { VK_RBUTTON } else { VK_LBUTTON })
}

// ---------- informations de fenêtre ----------
pub fn win_title(h: isize) -> String {
    let mut buf = [0u16; 512];
    let n = unsafe { GetWindowTextW(hwnd(h), buf.as_mut_ptr(), 512) };
    String::from_utf16_lossy(&buf[..n.max(0) as usize])
}

pub fn win_class(h: isize) -> String {
    let mut buf = [0u16; 256];
    let n = unsafe { GetClassNameW(hwnd(h), buf.as_mut_ptr(), 256) };
    String::from_utf16_lossy(&buf[..n.max(0) as usize])
}

pub fn win_exe(h: isize) -> String {
    unsafe {
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd(h), &mut pid);
        let ph = OpenProcess(0x1000, 0, pid); // PROCESS_QUERY_LIMITED_INFORMATION
        if ph.is_null() {
            return String::new();
        }
        let mut buf = [0u16; 1024];
        let mut n = 1024u32;
        let ok = QueryFullProcessImageNameW(ph, 0, buf.as_mut_ptr(), &mut n);
        CloseHandle(ph);
        if ok == 0 {
            return String::new();
        }
        let path = String::from_utf16_lossy(&buf[..n as usize]);
        path.rsplit(['\\', '/']).next().unwrap_or("").to_lowercase()
    }
}

pub fn win_rect(h: isize) -> (i32, i32, i32, i32) {
    let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
    unsafe {
        GetWindowRect(hwnd(h), &mut r);
    }
    (r.left, r.top, r.right, r.bottom)
}

pub fn foreground() -> isize {
    unsafe { GetForegroundWindow() as isize }
}

pub fn is_window(h: isize) -> bool {
    h != 0 && unsafe { IsWindow(hwnd(h)) != 0 }
}

pub fn cursor_pos() -> (i32, i32) {
    let mut p = POINT { x: 0, y: 0 };
    unsafe {
        GetCursorPos(&mut p);
    }
    (p.x, p.y)
}

pub fn set_cursor_pos(x: i32, y: i32) {
    unsafe {
        SetCursorPos(x, y);
    }
}

/// Fenêtre de premier niveau située sous un point de l'écran.
pub fn root_at(x: i32, y: i32) -> isize {
    unsafe {
        let h = WindowFromPoint(POINT { x, y });
        if h.is_null() {
            0
        } else {
            GetAncestor(h, GA_ROOT) as isize
        }
    }
}

/// Décrit la fenêtre visée par un clic en (x, y).
pub fn target_at(x: i32, y: i32) -> Option<(Target, isize)> {
    let h = root_at(x, y);
    if h == 0 {
        return None;
    }
    let (l, t, r, b) = win_rect(h);
    let (w, hh) = (r - l, b - t);
    Some((
        Target {
            hwnd: h,
            exe: win_exe(h),
            cls: win_class(h),
            title: win_title(h),
            dx: x - l,
            dy: y - t,
            w,
            h: hh,
            fx: (x - l) as f64 / w.max(1) as f64,
            fy: (y - t) as f64 / hh.max(1) as f64,
        },
        h,
    ))
}

// ---------- souris ----------
fn mouse_flag(flag: u32) -> bool {
    send_input(INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 { mi: MOUSEINPUT { dx: 0, dy: 0, mouseData: 0, dwFlags: flag, time: 0, dwExtraInfo: 0 } },
    })
}

fn mouse_click(x: i32, y: i32) -> Result<(), String> {
    set_cursor_pos(x, y);
    sleep(Duration::from_millis(50));
    if cursor_pos() != (x, y) {
        return Err(i18n::t(Msg::ErrCursor).to_string());
    }
    let down_ok = mouse_flag(MOUSEEVENTF_LEFTDOWN);
    sleep(Duration::from_millis(30));
    let up_ok = mouse_flag(MOUSEEVENTF_LEFTUP);
    sleep(Duration::from_millis(30));
    if down_ok && up_ok {
        Ok(())
    } else {
        Err(i18n::t(Msg::ErrClick).to_string())
    }
}

// ---------- retrouver, focaliser, cliquer ----------
unsafe extern "system" fn collect(h: HWND, lparam: LPARAM) -> i32 {
    let list = &mut *(lparam as *mut Vec<isize>);
    if IsWindowVisible(h) != 0 {
        list.push(h as isize);
    }
    1
}

/// Retrouve la fenêtre cible : même handle si elle existe encore, sinon même programme + même classe.
pub fn find_window(t: &Target) -> Option<isize> {
    if is_window(t.hwnd) && win_class(t.hwnd) == t.cls && win_exe(t.hwnd) == t.exe {
        return Some(t.hwnd);
    }
    let mut all: Vec<isize> = Vec::new();
    unsafe {
        EnumWindows(Some(collect), &mut all as *mut Vec<isize> as LPARAM);
    }
    let found: Vec<isize> = all.into_iter().filter(|h| win_class(*h) == t.cls && win_exe(*h) == t.exe).collect();
    found.iter().copied().find(|h| win_title(*h) == t.title).or_else(|| found.first().copied())
}

/// Amène la fenêtre au premier plan (la restaure si réduite). Vrai si réussi ; `stop` permet d'interrompre l'attente.
pub fn focus_window(h: isize, stop: &dyn Fn() -> bool) -> bool {
    unsafe {
        if IsIconic(hwnd(h)) != 0 {
            ShowWindowAsync(hwnd(h), SW_RESTORE); // asynchrone : ne bloque pas si la fenêtre ne répond pas
        }
        for attempt in 0..2 {
            let fg = GetForegroundWindow();
            if fg as isize == h {
                return true;
            }
            let cur = GetCurrentThreadId();
            let fg_thread = if fg.is_null() { 0 } else { GetWindowThreadProcessId(fg, std::ptr::null_mut()) };
            if attempt > 0 {
                // 2e essai : touche Alt « neutre » pour lever le blocage de Windows
                keybd_event(0x12, 0, 0, 0);
                keybd_event(0x12, 0, 2, 0);
            }
            if fg_thread != 0 && fg_thread != cur {
                AttachThreadInput(cur, fg_thread, 1);
            }
            BringWindowToTop(hwnd(h));
            SetForegroundWindow(hwnd(h));
            if fg_thread != 0 && fg_thread != cur {
                AttachThreadInput(cur, fg_thread, 0);
            }
            for _ in 0..20 {
                sleep(Duration::from_millis(50));
                if stop() {
                    return false;
                }
                if GetForegroundWindow() as isize == h {
                    return true;
                }
            }
        }
    }
    false
}

/// Focalise la fenêtre cible puis clique dans la zone repérée.
pub fn click_target(t: &Target, stop: &dyn Fn() -> bool) -> Result<(), String> {
    let h = find_window(t).ok_or_else(|| i18n::t(Msg::ErrWindowNotFound).to_string())?;
    if !focus_window(h, stop) {
        return Err(if stop() { i18n::t(Msg::ErrCancelled).to_string() } else { i18n::t(Msg::ErrForeground).to_string() });
    }
    let (l, tp, r, b) = win_rect(h);
    let (w, hgt) = (r - l, b - tp);
    let (x, y) = if (w, hgt) == (t.w, t.h) {
        (l + t.dx, tp + t.dy)
    } else {
        // fenêtre redimensionnée : position proportionnelle
        (l + (t.fx * w as f64) as i32, tp + (t.fy * hgt as f64) as i32)
    };
    if root_at(x, y) != h {
        return Err(i18n::t(Msg::ErrCovered).to_string());
    }
    mouse_click(x, y)?;
    sleep(Duration::from_millis(150));
    Ok(())
}

const SS_CENTER: u32 = 0x1; // styles du contrôle STATIC
const SS_CENTERIMAGE: u32 = 0x200;

// ---------- ouvrir un lien ----------
/// Ouvre une adresse web dans le navigateur par défaut (appel standard de Windows, sans passer par l'Explorateur).
pub fn open_url(url: &str) {
    let wide = |s: &str| -> Vec<u16> { s.encode_utf16().chain(std::iter::once(0)).collect() };
    unsafe {
        ShellExecuteW(std::ptr::null_mut(), wide("open").as_ptr(), wide(url).as_ptr(), std::ptr::null(), std::ptr::null(), 1);
    }
}

// ---------- instance unique ----------
/// Vrai si une autre instance d'AutoKey tourne déjà ; dans ce cas sa fenêtre est ramenée au premier plan.
/// (Deux instances armées taperaient deux fois et s'écraseraient les réglages.)
pub fn another_instance_running(window_title: &str) -> bool {
    let wide = |s: &str| -> Vec<u16> { s.encode_utf16().chain(std::iter::once(0)).collect() };
    unsafe {
        // le handle n'est jamais fermé : le mutex vit aussi longtemps que le processus
        CreateMutexW(std::ptr::null(), 0, wide("Local\\AutoKey.SingleInstance").as_ptr());
        if GetLastError() != ERROR_ALREADY_EXISTS {
            return false;
        }
        let other = FindWindowW(std::ptr::null(), wide(window_title).as_ptr());
        if !other.is_null() {
            ShowWindowAsync(other, SW_RESTORE);
            SetForegroundWindow(other);
        }
        true
    }
}

// ---------- identité du processus ----------
pub fn own_pid() -> u32 {
    unsafe { GetCurrentProcessId() }
}

pub fn win_pid(h: isize) -> u32 {
    let mut pid = 0u32;
    unsafe {
        GetWindowThreadProcessId(hwnd(h), &mut pid);
    }
    pid
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
        let thread = std::thread::spawn(move || unsafe { banner_thread(&text, &c2) });
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

unsafe fn banner_thread(text: &str, close: &AtomicBool) {
    let scale = GetDpiForSystem() as f32 / 96.0;
    let wide = |s: &str| -> Vec<u16> { s.encode_utf16().chain(std::iter::once(0)).collect() };
    let (class, title) = (wide("STATIC"), wide(text));
    let hwnd = CreateWindowExW(
        WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
        class.as_ptr(),
        title.as_ptr(),
        WS_POPUP | WS_VISIBLE | WS_BORDER | SS_CENTER | SS_CENTERIMAGE,
        (20.0 * scale) as i32,
        (20.0 * scale) as i32,
        (680.0 * scale) as i32,
        (60.0 * scale) as i32,
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        std::ptr::null(),
    );
    let font = CreateFontW(-((22.0 * scale) as i32), 0, 0, 0, 700, 0, 0, 0, 1, 0, 0, 5, 0, wide("Segoe UI").as_ptr());
    if !hwnd.is_null() {
        SendMessageW(hwnd, WM_SETFONT, font as usize, 1);
    }
    let mut msg: MSG = std::mem::zeroed();
    while !close.load(Ordering::SeqCst) {
        while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        sleep(Duration::from_millis(20));
    }
    if !hwnd.is_null() {
        DestroyWindow(hwnd);
    }
    DeleteObject(font);
}
