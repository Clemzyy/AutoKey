//! Moteur : envoi de touches, fenêtres, souris, anti-veille. Aucune interface ici.
//!
//! Une implémentation par système (`windows`, `linux`, `macos`) fournit les mêmes fonctions ; la logique qui ne dépend
//! pas du système (repérer une zone, la retrouver, cliquer dedans) est écrite une seule fois ici.
use std::thread::sleep;
use std::time::Duration;

use crate::i18n::{self, Msg};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

#[cfg(target_os = "linux")]
pub use linux::*;
#[cfg(target_os = "macos")]
pub use macos::*;
#[cfg(windows)]
pub use windows::*;

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
