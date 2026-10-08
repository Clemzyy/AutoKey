//! Disposition AZERTY : étiquette, code de balayage (touche physique), touche étendue, largeur relative.

pub struct Key {
    pub label: &'static str,
    pub scan: u16,
    pub ext: bool,
    pub w: f32,
}

const fn k(label: &'static str, scan: u16, w: f32, ext: bool) -> Key {
    Key { label, scan, ext, w }
}

pub const ROWS: [&[Key]; 6] = [
    &[
        k("Échap", 0x01, 1.5, false), k("F1", 0x3B, 1.0, false), k("F2", 0x3C, 1.0, false),
        k("F3", 0x3D, 1.0, false), k("F4", 0x3E, 1.0, false), k("F5", 0x3F, 1.0, false),
        k("F6", 0x40, 1.0, false), k("F7", 0x41, 1.0, false), k("F8", 0x42, 1.0, false),
        k("F9", 0x43, 1.0, false), k("F10", 0x44, 1.0, false), k("F11", 0x57, 1.0, false),
        k("F12", 0x58, 1.0, false),
    ],
    &[
        k("²", 0x29, 1.0, false), k("&", 0x02, 1.0, false), k("é", 0x03, 1.0, false),
        k("\"", 0x04, 1.0, false), k("'", 0x05, 1.0, false), k("(", 0x06, 1.0, false),
        k("-", 0x07, 1.0, false), k("è", 0x08, 1.0, false), k("_", 0x09, 1.0, false),
        k("ç", 0x0A, 1.0, false), k("à", 0x0B, 1.0, false), k(")", 0x0C, 1.0, false),
        k("=", 0x0D, 1.0, false), k("⌫", 0x0E, 2.0, false),
    ],
    &[
        k("Tab", 0x0F, 1.5, false), k("A", 0x10, 1.0, false), k("Z", 0x11, 1.0, false),
        k("E", 0x12, 1.0, false), k("R", 0x13, 1.0, false), k("T", 0x14, 1.0, false),
        k("Y", 0x15, 1.0, false), k("U", 0x16, 1.0, false), k("I", 0x17, 1.0, false),
        k("O", 0x18, 1.0, false), k("P", 0x19, 1.0, false), k("^", 0x1A, 1.0, false),
        k("$", 0x1B, 1.0, false), k("Entrée", 0x1C, 1.5, false),
    ],
    &[
        k("Verr.Maj", 0x3A, 1.8, false), k("Q", 0x1E, 1.0, false), k("S", 0x1F, 1.0, false),
        k("D", 0x20, 1.0, false), k("F", 0x21, 1.0, false), k("G", 0x22, 1.0, false),
        k("H", 0x23, 1.0, false), k("J", 0x24, 1.0, false), k("K", 0x25, 1.0, false),
        k("L", 0x26, 1.0, false), k("M", 0x27, 1.0, false), k("ù", 0x28, 1.0, false),
        k("*", 0x2B, 1.0, false),
    ],
    &[
        k("Maj", 0x2A, 1.3, false), k("<", 0x56, 1.0, false), k("W", 0x2C, 1.0, false),
        k("X", 0x2D, 1.0, false), k("C", 0x2E, 1.0, false), k("V", 0x2F, 1.0, false),
        k("B", 0x30, 1.0, false), k("N", 0x31, 1.0, false), k(",", 0x32, 1.0, false),
        k(";", 0x33, 1.0, false), k(":", 0x34, 1.0, false), k("!", 0x35, 1.0, false),
        k("Maj ", 0x36, 2.2, false),
    ],
    &[
        k("Ctrl", 0x1D, 1.5, false), k("Alt", 0x38, 1.5, false), k("Espace", 0x39, 7.0, false),
        k("Alt Gr", 0x38, 1.5, true), k("←", 0x4B, 1.0, true), k("↑", 0x48, 1.0, true),
        k("↓", 0x50, 1.0, true), k("→", 0x4D, 1.0, true),
    ],
];

/// Modificateurs maintenus pendant l'envoi des autres touches.
pub const MODS: [&str; 5] = ["Maj", "Maj ", "Ctrl", "Alt", "Alt Gr"];

pub fn is_mod(label: &str) -> bool {
    MODS.contains(&label)
}

/// Touche (scan, ext) d'un modificateur.
pub fn mod_key(label: &str) -> Option<(u16, bool)> {
    ROWS.iter().flat_map(|r| r.iter()).find(|k| k.label == label && is_mod(k.label)).map(|k| (k.scan, k.ext))
}

/// Recherche une touche par son nom, tel qu'écrit entre crochets (insensible à la casse).
pub fn lookup(name: &str) -> Option<(u16, bool)> {
    let n = name.trim().to_lowercase();
    let alias = match n.as_str() {
        "enter" => "entrée",
        "esc" => "échap",
        "backspace" => "⌫",
        "space" => "espace",
        other => other,
    };
    ROWS.iter()
        .flat_map(|r| r.iter())
        .find(|k| !is_mod(k.label) && k.label.trim().to_lowercase() == alias)
        .map(|k| (k.scan, k.ext))
}
