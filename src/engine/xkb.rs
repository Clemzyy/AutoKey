//! Disposition de clavier lue avec libxkbcommon (chargée à l'exécution, comme le fait déjà la fenêtre de l'application).
//!
//! Sous Wayland, le serveur X ne décrit pas la disposition active et le portail n'ajoute ni Maj ni Alt Gr : AutoKey
//! retrouve donc lui-même la touche de chaque caractère.
use std::ffi::CString;
use std::ptr;

use xkbcommon_dl::{xkb_context_flags, xkb_keymap_compile_flags, xkb_rule_names, xkbcommon_option};

/// Symboles de chaque touche, au format du protocole X11 : rang 0 = base, 1 = Maj, 4 = Alt Gr, 5 = Alt Gr + Maj.
/// Renvoie (premier code de touche, nombre de rangs par touche, symboles). Les codes sont ceux d'évdev + 8.
pub fn build(layout: &str, variant: &str) -> Option<(u8, usize, Vec<u32>)> {
    const PER: usize = 8;
    let xkb = xkbcommon_option()?;
    let layout = CString::new(layout).ok()?;
    let variant = CString::new(variant).ok()?;
    unsafe {
        let context = (xkb.xkb_context_new)(xkb_context_flags::XKB_CONTEXT_NO_FLAGS);
        if context.is_null() {
            return None;
        }
        let names = xkb_rule_names {
            rules: ptr::null(),
            model: ptr::null(),
            layout: layout.as_ptr(),
            variant: variant.as_ptr(),
            options: ptr::null(),
        };
        let keymap = (xkb.xkb_keymap_new_from_names)(context, &names, xkb_keymap_compile_flags::XKB_KEYMAP_COMPILE_NO_FLAGS);
        if keymap.is_null() {
            (xkb.xkb_context_unref)(context);
            return None;
        }
        let first = (xkb.xkb_keymap_min_keycode)(keymap).max(8);
        let last = (xkb.xkb_keymap_max_keycode)(keymap).min(255);
        let mut syms = vec![0u32; (last.saturating_sub(first) as usize + 1) * PER];
        for key in first..=last {
            // niveaux xkb : 0 = base, 1 = Maj, 2 = Alt Gr, 3 = Alt Gr + Maj
            for (level, rank) in [(0u32, 0usize), (1, 1), (2, 4), (3, 5)] {
                let mut out: *const u32 = ptr::null();
                let n = (xkb.xkb_keymap_key_get_syms_by_level)(keymap, key, 0, level, &mut out);
                if n >= 1 && !out.is_null() {
                    syms[(key - first) as usize * PER + rank] = *out;
                }
            }
        }
        (xkb.xkb_keymap_unref)(keymap);
        (xkb.xkb_context_unref)(context);
        Some((first as u8, PER, syms))
    }
}
