//! Touches : noms, modificateurs et dispositions de clavier (AZERTY, QWERTY, QWERTZ, russe, arabe…).
//!
//! Une touche entre crochets (`[a]`, `[Enter]`) désigne soit une touche nommée (position physique fixe),
//! soit un caractère, traduit à l'envoi selon la disposition réellement active dans Windows.
use crate::i18n::{self, Msg};

/// Touche nommée : sa position physique ne dépend pas de la disposition du clavier.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Named {
    Esc,
    F(u8),
    Tab,
    Caps,
    Enter,
    Backspace,
    Space,
    Left,
    Up,
    Down,
    Right,
    Delete,
    Insert,
    Home,
    End,
    PageUp,
    PageDown,
}

impl Named {
    /// (code de balayage, touche étendue)
    pub fn scan(self) -> (u16, bool) {
        match self {
            Named::Esc => (0x01, false),
            Named::F(n) if (1..=10).contains(&n) => (0x3A + n as u16, false),
            Named::F(11) => (0x57, false),
            Named::F(_) => (0x58, false),
            Named::Tab => (0x0F, false),
            Named::Caps => (0x3A, false),
            Named::Enter => (0x1C, false),
            Named::Backspace => (0x0E, false),
            Named::Space => (0x39, false),
            Named::Left => (0x4B, true),
            Named::Up => (0x48, true),
            Named::Down => (0x50, true),
            Named::Right => (0x4D, true),
            Named::Delete => (0x53, true),
            Named::Insert => (0x52, true),
            Named::Home => (0x47, true),
            Named::End => (0x4F, true),
            Named::PageUp => (0x49, true),
            Named::PageDown => (0x51, true),
        }
    }

    /// Nom écrit entre crochets dans le texte (toujours en anglais, quelle que soit la langue de l'interface).
    pub fn canonical(self) -> String {
        match self {
            Named::Esc => "Esc".into(),
            Named::F(n) => format!("F{n}"),
            Named::Tab => "Tab".into(),
            Named::Caps => "CapsLock".into(),
            Named::Enter => "Enter".into(),
            Named::Backspace => "Backspace".into(),
            Named::Space => "Space".into(),
            Named::Left => "Left".into(),
            Named::Up => "Up".into(),
            Named::Down => "Down".into(),
            Named::Right => "Right".into(),
            Named::Delete => "Delete".into(),
            Named::Insert => "Insert".into(),
            Named::Home => "Home".into(),
            Named::End => "End".into(),
            Named::PageUp => "PageUp".into(),
            Named::PageDown => "PageDown".into(),
        }
    }

    /// Texte affiché sur la touche du clavier à l'écran.
    pub fn label(self) -> String {
        match self {
            Named::Esc => i18n::t(Msg::KeyEsc).into(),
            Named::Caps => i18n::t(Msg::KeyCaps).into(),
            Named::Enter => i18n::t(Msg::KeyEnter).into(),
            Named::Space => i18n::t(Msg::KeySpace).into(),
            Named::Backspace => "⌫".into(),
            Named::Left => "←".into(),
            Named::Up => "↑".into(),
            Named::Down => "↓".into(),
            Named::Right => "→".into(),
            other => other.canonical(),
        }
    }

    /// Reconnaît un nom entre crochets : anglais, français et libellés de toutes les langues de l'interface.
    fn parse(name: &str) -> Option<Named> {
        let n = name.trim().to_lowercase();
        if let Some(num) = n.strip_prefix('f').and_then(|x| x.parse::<u8>().ok()) {
            if (1..=12).contains(&num) {
                return Some(Named::F(num));
            }
        }
        let is = |list: &[&str]| list.contains(&n.as_str());
        Some(if is(&["esc", "escape", "échap", "echap", "退出"]) {
            Named::Esc
        } else if is(&["tab", "tabulation", "tabulador"]) {
            Named::Tab
        } else if is(&["capslock", "caps lock", "caps", "verr.maj", "verr maj", "bloq mayús", "bloq mayus", "大写锁定"]) {
            Named::Caps
        } else if is(&["enter", "return", "entrée", "entree", "intro", "回车"]) {
            Named::Enter
        } else if is(&["backspace", "⌫", "retour arrière", "retroceso"]) {
            Named::Backspace
        } else if is(&["space", "espace", "espacio", "пробел", "مسافة", "空格"]) {
            Named::Space
        } else if is(&["left", "←", "gauche"]) {
            Named::Left
        } else if is(&["up", "↑", "haut"]) {
            Named::Up
        } else if is(&["down", "↓", "bas"]) {
            Named::Down
        } else if is(&["right", "→", "droite"]) {
            Named::Right
        } else if is(&["delete", "del", "suppr", "supprimer"]) {
            Named::Delete
        } else if is(&["insert", "ins", "inser"]) {
            Named::Insert
        } else if is(&["home", "début", "debut", "origine"]) {
            Named::Home
        } else if is(&["end", "fin"]) {
            Named::End
        } else if is(&["pageup", "page up", "pgup", "page précédente"]) {
            Named::PageUp
        } else if is(&["pagedown", "page down", "pgdn", "page suivante"]) {
            Named::PageDown
        } else {
            return None;
        })
    }
}

/// Modificateur maintenu pendant l'envoi.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Modk {
    LShift,
    RShift,
    Ctrl,
    Alt,
    AltGr,
}

impl Modk {
    pub fn scan(self) -> (u16, bool) {
        match self {
            Modk::LShift => (0x2A, false),
            Modk::RShift => (0x36, false),
            Modk::Ctrl => (0x1D, false),
            Modk::Alt => (0x38, false),
            Modk::AltGr => (0x38, true),
        }
    }

    /// Nom enregistré dans les réglages.
    pub fn canonical(self) -> &'static str {
        match self {
            Modk::LShift => "shift",
            Modk::RShift => "rshift",
            Modk::Ctrl => "ctrl",
            Modk::Alt => "alt",
            Modk::AltGr => "altgr",
        }
    }

    pub fn label(self) -> String {
        match self {
            Modk::LShift | Modk::RShift => i18n::t(Msg::KeyShift).into(),
            Modk::Ctrl => "Ctrl".into(),
            Modk::Alt => "Alt".into(),
            Modk::AltGr => "Alt Gr".into(),
        }
    }

    /// Accepte les noms actuels et ceux des anciens réglages (« Maj », « Maj  », « Alt Gr »…).
    pub fn parse(name: &str) -> Option<Modk> {
        Some(match name {
            "shift" | "Maj" => Modk::LShift,
            "rshift" | "Maj " => Modk::RShift,
            "ctrl" | "Ctrl" => Modk::Ctrl,
            "alt" | "Alt" => Modk::Alt,
            "altgr" | "Alt Gr" => Modk::AltGr,
            _ => return None,
        })
    }
}

/// Ce que produit une touche cliquée.
#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    Char(char),
    Named(Named),
    Mod(Modk),
    /// Touche produisant plusieurs caractères (ex. « لا ») : insérée comme texte ordinaire.
    Text(String),
}

/// Une touche du clavier à l'écran.
#[derive(Clone, Debug)]
pub struct KeyDef {
    pub label: String,
    pub tok: Tok,
    pub w: f32,
}

/// Reconnaît ce qui est écrit entre crochets : touche nommée ou caractère unique.
pub fn lookup(name: &str) -> Option<Tok> {
    if let Some(n) = Named::parse(name) {
        return Some(Tok::Named(n));
    }
    let mut chars = name.trim().chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => Some(Tok::Char(c.to_lowercase().next().unwrap_or(c))),
        _ => None,
    }
}

// ---------- dispositions ----------
pub struct KbLayout {
    pub id: &'static str,
    pub name: &'static str,
    /// Clavier ISO (touche supplémentaire à gauche de W/Z, grande touche Entrée) ou ANSI.
    pub iso: bool,
    /// Rangées (sans les touches de contrôle) : chiffres, 1re, 2e et 3e rangée de lettres, labels séparés par des espaces.
    pub rows: [&'static str; 4],
    /// Identifiants de langue Windows (octet de poids faible) qui utilisent cette disposition.
    pub langids: &'static [u16],
}

pub const LAYOUTS: &[KbLayout] = &[
    KbLayout { id: "azerty-fr", name: "AZERTY – FR", iso: true, langids: &[0x040C, 0x0C0C, 0x140C],
        rows: ["² & é \" ' ( - è _ ç à ) =", "a z e r t y u i o p ^ $", "q s d f g h j k l m ù *", "< w x c v b n , ; : !"] },
    KbLayout { id: "azerty-be", name: "AZERTY – BE", iso: true, langids: &[0x080C, 0x0813],
        rows: ["² & é \" ' ( § è ! ç à ) -", "a z e r t y u i o p ^ $", "q s d f g h j k l m ù µ", "< w x c v b n , ; : ="] },
    KbLayout { id: "qwerty-us", name: "QWERTY – US", iso: false, langids: &[0x0409, 0x1009, 0x0C09, 0x1409],
        rows: ["` 1 2 3 4 5 6 7 8 9 0 - =", "q w e r t y u i o p [ ] \\", "a s d f g h j k l ; '", "z x c v b n m , . /"] },
    KbLayout { id: "qwerty-uk", name: "QWERTY – UK", iso: true, langids: &[0x0809],
        rows: ["` 1 2 3 4 5 6 7 8 9 0 - =", "q w e r t y u i o p [ ]", "a s d f g h j k l ; ' #", "\\ z x c v b n m , . /"] },
    KbLayout { id: "qwertz-de", name: "QWERTZ – DE", iso: true, langids: &[0x0407, 0x0C07, 0x1407],
        rows: ["^ 1 2 3 4 5 6 7 8 9 0 ß ´", "q w e r t z u i o p ü +", "a s d f g h j k l ö ä #", "< y x c v b n m , . -"] },
    KbLayout { id: "qwertz-ch", name: "QWERTZ – CH", iso: true, langids: &[0x0807, 0x100C],
        rows: ["§ 1 2 3 4 5 6 7 8 9 0 ' ^", "q w e r t z u i o p ü ¨", "a s d f g h j k l ö ä $", "< y x c v b n m , . -"] },
    KbLayout { id: "qwerty-es", name: "QWERTY – ES", iso: true, langids: &[0x040A, 0x0C0A],
        rows: ["º 1 2 3 4 5 6 7 8 9 0 ' ¡", "q w e r t y u i o p ` +", "a s d f g h j k l ñ ´ ç", "< z x c v b n m , . -"] },
    KbLayout { id: "qwerty-it", name: "QWERTY – IT", iso: true, langids: &[0x0410],
        rows: ["\\ 1 2 3 4 5 6 7 8 9 0 ' ì", "q w e r t y u i o p è +", "a s d f g h j k l ò à ù", "< z x c v b n m , . -"] },
    KbLayout { id: "qwerty-br", name: "QWERTY – BR", iso: true, langids: &[0x0416, 0x0816],
        rows: ["' 1 2 3 4 5 6 7 8 9 0 - =", "q w e r t y u i o p ´ [", "a s d f g h j k l ç ~ ]", "\\ z x c v b n m , . ;"] },
    KbLayout { id: "ru", name: "ЙЦУКЕН – RU", iso: true, langids: &[0x0419, 0x0422, 0x0423],
        rows: ["ё 1 2 3 4 5 6 7 8 9 0 - =", "й ц у к е н г ш щ з х ъ", "ф ы в а п р о л д ж э \\", "/ я ч с м и т ь б ю ."] },
    KbLayout { id: "ar", name: "العربية", iso: false, langids: &[0x0401, 0x0801, 0x0C01, 0x1001, 0x1401, 0x1801, 0x1C01, 0x2001, 0x2401, 0x2801, 0x2C01, 0x3001, 0x3401, 0x3801, 0x3C01, 0x4001],
        rows: ["ذ 1 2 3 4 5 6 7 8 9 0 - =", "ض ص ث ق ف غ ع ه خ ح ج د \\", "ش س ي ب ل ا ت ن م ك ط", "ئ ء ؤ ر لا ى ة و ز ظ"] },
    KbLayout { id: "dvorak", name: "Dvorak", iso: false, langids: &[],
        rows: ["` 1 2 3 4 5 6 7 8 9 0 [ ]", "' , . p y f g c r l / = \\", "a o e u i d h t n s -", "; q j k x b m w v z"] },
    KbLayout { id: "colemak", name: "Colemak", iso: false, langids: &[],
        rows: ["` 1 2 3 4 5 6 7 8 9 0 - =", "q w f p g j l u y ; [ ] \\", "a r s t d h n e i o '", "z x c v b k m , . /"] },
];

pub fn by_id(id: &str) -> &'static KbLayout {
    LAYOUTS.iter().find(|l| l.id == id).unwrap_or(&LAYOUTS[2])
}

/// Disposition correspondant à la langue de saisie active de Windows (le chinois, le japonais… utilisent le QWERTY US).
pub fn detect() -> &'static KbLayout {
    if let Some(l) = std::env::var("AUTOKEY_LAYOUT").ok().and_then(|id| LAYOUTS.iter().find(|l| l.id == id)) {
        return l;
    }
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetKeyboardLayout;
    let langid = (unsafe { GetKeyboardLayout(0) } as usize & 0xFFFF) as u16;
    LAYOUTS
        .iter()
        .find(|l| l.langids.contains(&langid))
        .or_else(|| if langid & 0x3FF == 0x01 { LAYOUTS.iter().find(|l| l.id == "ar") } else { None })
        .unwrap_or(&LAYOUTS[2])
}

fn key(label: &str, tok: Tok, w: f32) -> KeyDef {
    KeyDef { label: label.to_string(), tok, w }
}

fn named(n: Named, w: f32) -> KeyDef {
    KeyDef { label: n.label(), tok: Tok::Named(n), w }
}

fn modifier(m: Modk, w: f32) -> KeyDef {
    KeyDef { label: m.label(), tok: Tok::Mod(m), w }
}

/// Touches produisant des caractères, à partir d'une rangée « a z e r t… ».
fn chars(row: &str) -> Vec<KeyDef> {
    row.split_whitespace()
        .map(|lab| {
            let mut it = lab.chars();
            match (it.next(), it.next()) {
                (Some(c), None) => key(&c.to_uppercase().collect::<String>(), Tok::Char(c), 1.0),
                _ => key(lab, Tok::Text(lab.to_string()), 1.0),
            }
        })
        .collect()
}

/// Les six rangées du clavier à l'écran pour une disposition, avec les libellés dans la langue courante.
pub fn build(l: &KbLayout) -> Vec<Vec<KeyDef>> {
    let mut r0 = vec![named(Named::Esc, 1.5)];
    r0.extend((1..=12).map(|n| named(Named::F(n), 1.0)));

    let mut r1 = chars(l.rows[0]);
    r1.push(named(Named::Backspace, 2.0));

    let mut r2 = vec![named(Named::Tab, 1.5)];
    r2.extend(chars(l.rows[1]));
    let mut r3 = vec![named(Named::Caps, 1.8)];
    r3.extend(chars(l.rows[2]));
    let mut r4 = vec![modifier(Modk::LShift, if l.iso { 1.3 } else { 2.3 })];
    r4.extend(chars(l.rows[3]));
    r4.push(modifier(Modk::RShift, if l.iso { 2.2 } else { 2.7 }));
    if l.iso {
        r2.push(named(Named::Enter, 1.5)); // grande touche Entrée : en fin de 2e rangée
    } else {
        r3.push(named(Named::Enter, 2.2));
    }

    let r5 = vec![
        modifier(Modk::Ctrl, 1.5),
        modifier(Modk::Alt, 1.5),
        named(Named::Space, 7.0),
        modifier(Modk::AltGr, 1.5),
        named(Named::Left, 1.0),
        named(Named::Up, 1.0),
        named(Named::Down, 1.0),
        named(Named::Right, 1.0),
    ];
    vec![r0, r1, r2, r3, r4, r5]
}
