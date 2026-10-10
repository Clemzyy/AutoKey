//! Données : actions planifiées, analyse du texte, calcul de l'heure, réglages sauvegardés.
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Datelike, Duration, Local, LocalResult, NaiveDate, NaiveDateTime, TimeZone, Timelike};
use serde_json::{json, Value};

use crate::engine::Target;
use crate::i18n::{self, tf, Msg};
use crate::keys::{self, Modk, Tok};

/// Une action planifiée : quand, quoi, où, et les 3 options.
#[derive(Clone, Debug, PartialEq)]
pub struct Action {
    pub h: u32,
    pub m: u32,
    pub s: u32,
    pub ms: u32,
    pub use_date: bool,
    pub date: String, // JJ/MM/AAAA
    pub text: String,
    pub mods: Vec<String>,
    pub repeat: bool,
    pub rep: u32,
    pub gap: u32, // ms
    pub target: Option<Target>,
    pub use_target: bool, // « aller dans la zone avant d'écrire »
    pub back: bool,       // « revenir ensuite où j'étais »
    pub minim: bool,      // « réduire la fenêtre au lancement »
}

impl Default for Action {
    fn default() -> Self {
        let now = Local::now();
        let t = now + Duration::minutes(1);
        Action {
            h: t.hour(),
            m: t.minute(),
            s: 0,
            ms: 0,
            use_date: false,
            date: now.format("%d/%m/%Y").to_string(),
            text: String::new(),
            mods: vec![],
            repeat: false,
            rep: 10,
            gap: 100,
            target: None,
            use_target: true,
            back: true,
            minim: true,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Item {
    /// Caractère tapé tel quel (Unicode) : accents, symboles, toute écriture.
    Char(char),
    /// Touche nommée (Entrée, Tab, F5…) : position physique fixe.
    Key(u16, bool),
    /// Touche d'un caractère (`[a]`) : traduite selon la disposition active de Windows, donc utilisable avec Ctrl, Alt…
    Press(char),
}

/// Action prête à être exécutée par le thread d'envoi.
#[derive(Clone, Debug)]
pub struct Job {
    pub ts: f64, // secondes depuis 1970
    pub items: Vec<Item>,
    pub mods: Vec<(u16, bool)>,
    pub n: u32,
    pub gap: f64,
    pub target: Option<Target>,
    pub back: bool,
    pub minim: bool,
}

pub fn now_secs() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
}

/// « bonjour[Entrée] » -> lettres + touches. Erreur si une touche entre crochets est inconnue.
/// Un « [ » sans « ] » correspondant (ou suivi d'un autre « [ ») est tapé tel quel.
pub fn parse(text: &str) -> Result<Vec<Item>, String> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '[' {
            // le nom s'arrête au premier « ] » ; s'il contient un autre « [ », le « [ » courant est un simple caractère
            let end = chars[i + 1..].iter().position(|&c| c == ']');
            if let Some(len) = end.filter(|&len| len > 0 && !chars[i + 1..i + 1 + len].contains(&'[')) {
                let name: String = chars[i + 1..i + 1 + len].iter().collect();
                match keys::lookup(&name) {
                    Some(Tok::Named(n)) => {
                        let (scan, ext) = n.scan();
                        out.push(Item::Key(scan, ext));
                    }
                    Some(Tok::Char(c)) => out.push(Item::Press(c)),
                    _ => return Err(tf(Msg::UnknownKey, &[&name])),
                }
                i += len + 2;
                continue;
            }
        }
        out.push(Item::Char(chars[i]));
        i += 1;
    }
    Ok(out)
}

/// Heure locale correspondant à une date/heure « murale » : prend la première des deux heures en cas
/// d'heure ambiguë (fin d'heure d'été) et décale d'une heure si elle n'existe pas (début d'heure d'été).
fn local_from_naive(naive: NaiveDateTime) -> Option<DateTime<Local>> {
    match Local.from_local_datetime(&naive) {
        LocalResult::Single(t) => Some(t),
        LocalResult::Ambiguous(first, _) => Some(first),
        LocalResult::None => Local.from_local_datetime(&(naive + Duration::hours(1))).earliest(),
    }
}

/// Prochain instant correspondant à l'heure (et à la date si demandée) d'une action.
pub fn when(a: &Action) -> Result<f64, String> {
    let now = Local::now();
    let date = if a.use_date {
        NaiveDate::parse_from_str(a.date.trim(), "%d/%m/%Y").or_else(|_| NaiveDate::parse_from_str(a.date.trim(), "%Y-%m-%d")).map_err(|_| i18n::t(Msg::DateFormatInvalid).to_string())?
    } else {
        now.date_naive()
    };
    let naive = date.and_hms_milli_opt(a.h, a.m, a.s, a.ms).ok_or_else(|| i18n::t(Msg::TimeInvalid).to_string())?;
    let mut t = local_from_naive(naive).ok_or_else(|| i18n::t(Msg::LocalTimeNotFound).to_string())?;
    if t <= now {
        if a.use_date {
            return Err(i18n::t(Msg::DatePassed).to_string());
        }
        let tomorrow = date.succ_opt().ok_or_else(|| i18n::t(Msg::DateFormatInvalid).to_string())?;
        t = local_from_naive(tomorrow.and_hms_milli_opt(a.h, a.m, a.s, a.ms).ok_or_else(|| i18n::t(Msg::TimeInvalid).to_string())?)
            .ok_or_else(|| i18n::t(Msg::LocalTimeNotFound).to_string())?;
    }
    Ok(t.timestamp() as f64 + t.timestamp_subsec_millis() as f64 / 1000.0)
}

/// Transforme une action en tâche exécutable. `ts` impose l'instant (utile pour le test).
pub fn make_job(a: &Action, ts: Option<f64>) -> Result<Job, String> {
    Ok(Job {
        ts: match ts {
            Some(t) => t,
            None => when(a)?,
        },
        items: parse(&a.text)?,
        mods: a.mods.iter().filter_map(|m| Modk::parse(m)).map(Modk::scan).collect(),
        n: if a.repeat { a.rep.max(1) } else { 1 },
        gap: if a.repeat { a.gap as f64 / 1000.0 } else { 0.0 },
        target: if a.use_target && crate::engine::targets_supported() { a.target.clone() } else { None },
        back: a.back,
        minim: a.minim,
    })
}

// ---------- réglages (même format que la version Python) ----------
/// Dossier de configuration de l'utilisateur : %APPDATA% (Windows), ~/Library/Application Support (macOS), ~/.config (Linux).
pub fn config_dir() -> PathBuf {
    let home = || std::env::var_os("HOME").map(PathBuf::from);
    let dir = if cfg!(windows) {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        home().map(|h| h.join("Library").join("Application Support"))
    } else {
        std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).or_else(|| home().map(|h| h.join(".config")))
    };
    dir.unwrap_or_else(|| PathBuf::from("."))
}

pub fn settings_path() -> PathBuf {
    if let Some(p) = std::env::var_os("AUTOKEY_SETTINGS") {
        return PathBuf::from(p); // tests : fichier de réglages alternatif
    }
    config_dir().join("AutoKey").join("reglages.json")
}

fn target_to_json(t: &Option<Target>) -> Value {
    match t {
        None => Value::Null,
        Some(t) => json!({"exe": t.exe, "cls": t.cls, "title": t.title, "dx": t.dx, "dy": t.dy,
                          "w": t.w, "h": t.h, "fx": t.fx, "fy": t.fy}),
    }
}

fn target_from_json(v: &Value) -> Option<Target> {
    let o = v.as_object()?;
    let s = |k: &str| o.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    let i = |k: &str| o.get(k).and_then(Value::as_i64).unwrap_or(0) as i32;
    let f = |k: &str| o.get(k).and_then(Value::as_f64).unwrap_or(0.0);
    if s("exe").is_empty() {
        return None;
    }
    Some(Target { hwnd: 0, exe: s("exe"), cls: s("cls"), title: s("title"), dx: i("dx"), dy: i("dy"), w: i("w"),
                  h: i("h"), fx: f("fx"), fy: f("fy") })
}

pub fn action_to_json(a: &Action) -> Value {
    json!({"h": a.h, "m": a.m, "s": a.s, "ms": a.ms, "use_date": a.use_date, "date": a.date, "text": a.text,
           "mods": a.mods, "repeat": a.repeat, "rep": a.rep, "gap": a.gap, "target": target_to_json(&a.target),
           "use_target": a.use_target, "back": a.back, "minim": a.minim})
}

/// Accepte un nombre (entier ou décimal) ou un texte contenant un nombre (la version Python sauvegarde « 10 » en texte).
fn num(v: Option<&Value>, default: u32) -> u32 {
    let n = match v {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => s.trim().parse::<f64>().ok(),
        _ => None,
    };
    n.filter(|x| x.is_finite()).map(|x| x.round().clamp(0.0, u32::MAX as f64) as u32).unwrap_or(default)
}

pub fn action_from_json(v: &Value, base: &Action) -> Action {
    let b = |k: &str, d: bool| v.get(k).and_then(Value::as_bool).unwrap_or(d);
    Action {
        h: num(v.get("h"), base.h).min(23),
        m: num(v.get("m"), base.m).min(59),
        s: num(v.get("s"), base.s).min(59),
        ms: num(v.get("ms"), base.ms).min(999),
        use_date: b("use_date", base.use_date),
        date: v.get("date").and_then(Value::as_str).map(str::to_string).unwrap_or_else(|| base.date.clone()),
        text: v.get("text").and_then(Value::as_str).unwrap_or("").to_string(),
        mods: v
            .get("mods")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).filter_map(|m| Modk::parse(&m).map(|k| k.canonical().to_string())).collect())
            .unwrap_or_default(),
        repeat: b("repeat", false),
        rep: num(v.get("rep"), 10).clamp(1, 9999),
        gap: num(v.get("gap"), 100).min(60_000),
        target: v.get("target").and_then(target_from_json),
        use_target: b("use_target", true),
        back: b("back", true),
        minim: b("minim", true),
    }
}

pub struct Saved {
    pub editor: Action,
    pub list_mode: bool,
    pub actions: Vec<Action>,
    pub lang: Option<String>,   // code de langue choisi par l'utilisateur (sinon : langue de Windows)
    pub layout: Option<String>, // identifiant de disposition choisi (sinon : détectée)
}

/// Écrit les réglages de façon atomique : fichier temporaire puis renommage, pour ne jamais laisser un JSON tronqué.
pub fn save(s: &Saved) {
    let mut v = action_to_json(&s.editor);
    // la version Python mémorise la répétition et l'intervalle en texte : on reste compatible
    v["rep"] = json!(s.editor.rep.to_string());
    v["gap"] = json!(s.editor.gap.to_string());
    v["list_mode"] = json!(s.list_mode);
    v["lang"] = json!(s.lang);
    v["layout"] = json!(s.layout);
    v["actions"] = Value::Array(s.actions.iter().map(action_to_json).collect());
    let path = settings_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(text) = serde_json::to_string_pretty(&v) {
        let tmp = path.with_extension("json.tmp");
        if std::fs::write(&tmp, text).is_ok() && std::fs::rename(&tmp, &path).is_err() {
            let _ = std::fs::remove_file(&tmp);
        }
    }
}

pub fn load() -> Saved {
    let base = Action::default();
    let empty = |base: Action| Saved { editor: base, list_mode: false, actions: vec![], lang: None, layout: None };
    let path = settings_path();
    let Ok(text) = std::fs::read_to_string(&path) else {
        return empty(base);
    };
    let Ok(v) = serde_json::from_str::<Value>(&text) else {
        // fichier illisible : on le garde de côté au lieu de l'écraser au prochain enregistrement
        let _ = std::fs::rename(&path, path.with_extension("json.bak"));
        return empty(base);
    };
    // l'heure n'est pas mémorisée : elle repart à « maintenant + 1 minute »
    let mut editor = action_from_json(&v, &base);
    editor.h = base.h;
    editor.m = base.m;
    editor.s = 0;
    editor.ms = 0;
    // une date mémorisée déjà passée n'a plus de sens : on repart d'aujourd'hui
    let today = Local::now().date_naive();
    if NaiveDate::parse_from_str(editor.date.trim(), "%d/%m/%Y").map_or(true, |d| d < today) {
        editor.date = format!("{:02}/{:02}/{}", today.day(), today.month(), today.year());
    }
    let actions = v
        .get("actions")
        .and_then(Value::as_array)
        .map(|a| a.iter().map(|x| action_from_json(x, &base)).collect())
        .unwrap_or_default();
    let text = |k: &str| v.get(k).and_then(Value::as_str).map(str::to_string);
    Saved { editor, list_mode: v.get("list_mode").and_then(Value::as_bool).unwrap_or(false), actions, lang: text("lang"), layout: text("layout") }
}
