//! Threads d'arrière-plan : envoi planifié des actions, et repérage de la zone de saisie à cibler.
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{sleep, yield_now};
use std::time::{Duration, Instant};

use chrono::{DateTime, Local};
use eframe::egui;

use crate::engine::{self, Target};
use crate::i18n::{self, tf, Msg};
use crate::model::{now_secs, Item, Job};

/// La fenêtre cible est préparée (restaurée, mise au premier plan, clic dans la zone) un peu avant l'heure,
/// pour que la première touche parte à l'heure exacte et non 300 ms plus tard.
const PREP_LEAD: f64 = 1.5;
/// Au-delà de ce retard (PC en veille, action précédente trop longue…), l'action est ignorée plutôt que
/// tapée au hasard dans la fenêtre active.
const MAX_LATE: f64 = 30.0;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Kind {
    Info,
    Wait,
    Ok,
    Warn,
    Err,
}

/// État partagé entre l'interface et les threads.
pub struct Shared {
    pub cancel: AtomicBool,
    pub running: AtomicBool,
    pub minimize: AtomicBool, // le thread demande de réduire la fenêtre
    pub restore: AtomicBool,  // le thread demande de la rouvrir (seulement s'il l'a réduite)
    pub status: Mutex<(String, Kind)>,
    pub picked: Mutex<Option<Option<Target>>>, // Some(None) = repérage annulé
    pub picking: AtomicBool,
}

/// Message affiché quand rien ne se passe : le rappel de l'arrêt d'urgence, ou l'avertissement Wayland sous Linux.
pub fn idle_status() -> (String, Kind) {
    match engine::session_warning() {
        Some(m) => (i18n::t(m).into(), Kind::Warn),
        None => (i18n::t(Msg::EmergencyStop).into(), Kind::Info),
    }
}

impl Shared {
    pub fn new() -> Arc<Self> {
        Arc::new(Shared {
            cancel: AtomicBool::new(false),
            running: AtomicBool::new(false),
            minimize: AtomicBool::new(false),
            restore: AtomicBool::new(false),
            status: Mutex::new(idle_status()),
            picked: Mutex::new(None),
            picking: AtomicBool::new(false),
        })
    }

    pub fn set_status(&self, ctx: &egui::Context, text: impl Into<String>, kind: Kind) {
        *self.status.lock().unwrap() = (text.into(), kind);
        ctx.request_repaint();
    }

    /// Vrai si l'envoi doit s'arrêter (bouton Annuler ou raccourci d'urgence).
    pub fn stop(&self) -> bool {
        if engine::panic_pressed() {
            self.cancel.store(true, Ordering::SeqCst);
        }
        self.cancel.load(Ordering::SeqCst)
    }

    /// Sommeil interruptible : sonde l'arrêt toutes les 15 ms. Renvoie faux si l'arrêt a été demandé.
    fn nap(&self, d: Duration) -> bool {
        let end = Instant::now() + d;
        loop {
            if self.stop() {
                return false;
            }
            let left = end.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return true;
            }
            sleep(left.min(Duration::from_millis(15)));
        }
    }
}

/// « 83210 s » -> « 23 h 06 min 50 s » (dans la langue courante).
fn fmt_left(left: f64) -> String {
    let t = left.ceil() as u64;
    if left >= 3600.0 {
        tf(Msg::CountdownHMS, &[&(t / 3600), &format!("{:02}", t % 3600 / 60), &format!("{:02}", t % 60)])
    } else if left >= 60.0 {
        tf(Msg::CountdownMS, &[&(t / 60), &format!("{:02}", t % 60)])
    } else if left >= 10.0 {
        tf(Msg::CountdownS, &[&t])
    } else {
        tf(Msg::CountdownS, &[&format!("{left:.1}")])
    }
}

/// Attend l'instant `ts` (secondes depuis 1970) : sommeil grossier, puis boucle serrée pour la précision
/// à la milliseconde. Renvoie faux si l'arrêt a été demandé.
fn wait_until(sh: &Shared, ctx: &egui::Context, ts: f64, name: &str) -> bool {
    let mut last_status = Instant::now() - Duration::from_secs(10);
    loop {
        if sh.stop() {
            return false;
        }
        let left = ts - now_secs();
        if left <= 0.0 {
            return true;
        }
        // loin de l'heure, inutile de redessiner l'interface 5 fois par seconde
        let every = if left > 120.0 { 1000 } else { 200 };
        if last_status.elapsed() >= Duration::from_millis(every) {
            sh.set_status(ctx, tf(Msg::WaitStatus, &[&name, &fmt_left(left)]), Kind::Wait);
            last_status = Instant::now();
        }
        if left > 0.1 {
            sleep(Duration::from_secs_f64((left - 0.05).min(0.02)));
        } else {
            yield_now();
        }
    }
}

/// Envoie une fois le texte/les touches. Renvoie faux si Windows a refusé au moins un événement.
fn press_once(job: &Job, sh: &Shared) -> bool {
    let mut ok = true;
    for (scan, ext) in &job.mods {
        ok &= engine::send_scan(*scan, *ext, false);
    }
    for item in &job.items {
        if sh.stop() {
            break;
        }
        match item {
            Item::Char(c) => ok &= engine::send_char(*c),
            Item::Press(c) => ok &= engine::press_char(*c),
            Item::Key(scan, ext) => {
                ok &= engine::send_scan(*scan, *ext, false);
                sleep(Duration::from_millis(20));
                ok &= engine::send_scan(*scan, *ext, true);
            }
        }
        sleep(Duration::from_millis(20));
    }
    // les modificateurs sont toujours relâchés, même après une annulation
    for (scan, ext) in job.mods.iter().rev() {
        engine::send_scan(*scan, *ext, true);
    }
    ok
}

/// Prépare la cible (clic dans la zone), attend l'heure exacte puis tape. Renvoie un message d'erreur éventuel.
fn run_job(job: &Job, sh: &Shared, ctx: &egui::Context, name: &str) -> Option<String> {
    let prev = engine::foreground();
    let cur0 = engine::cursor_pos();
    let mut err = None;
    if let Some(t) = &job.target {
        if let Err(e) = engine::click_target(t, &|| sh.stop()) {
            err = Some(e);
        }
    }
    if err.is_none() && wait_until(sh, ctx, job.ts, name) {
        for i in 0..job.n {
            if sh.stop() {
                break;
            }
            if !press_once(job, sh) {
                err = Some(i18n::t(Msg::KeysRefused).to_string());
                break;
            }
            if job.gap > 0.0 && i + 1 < job.n && !sh.nap(Duration::from_secs_f64(job.gap)) {
                break;
            }
        }
    }
    if job.target.is_some() && job.back {
        engine::set_cursor_pos(cur0.0, cur0.1);
        if engine::is_window(prev) {
            engine::focus_window(prev, &|| false); // même après une annulation : on rend la main à l'utilisateur
        }
    }
    err
}

pub fn spawn_jobs(jobs: Vec<Job>, label: Option<String>, sh: Arc<Shared>, ctx: egui::Context) {
    sh.cancel.store(false, Ordering::SeqCst);
    sh.running.store(true, Ordering::SeqCst);
    std::thread::spawn(move || {
        engine::keep_awake(true);
        let total = jobs.len();
        let (mut done, mut errors, mut minimized) = (0, Vec::<String>::new(), false);
        for (k, job) in jobs.iter().enumerate() {
            if job.minim {
                minimized = true;
                sh.minimize.store(true, Ordering::SeqCst);
                ctx.request_repaint();
            }
            let when: DateTime<Local> = DateTime::from_timestamp(job.ts as i64, 0).unwrap_or_default().into();
            let name = label.clone().unwrap_or_else(|| tf(Msg::ActionName, &[&(k + 1), &total, &when.format("%H:%M:%S")]));
            // on attend le moment de préparer la fenêtre cible (ou directement l'heure s'il n'y a pas de cible)
            let prep_at = if job.target.is_some() { job.ts - PREP_LEAD } else { job.ts };
            if !wait_until(&sh, &ctx, prep_at, &name) {
                break;
            }
            let late = now_secs() - job.ts;
            if late > MAX_LATE {
                errors.push(tf(Msg::Late, &[&(k + 1), &format!("{late:.0}")]));
                continue;
            }
            match run_job(job, &sh, &ctx, &name) {
                Some(e) => errors.push(tf(Msg::ActionErr, &[&(k + 1), &e])),
                None if !sh.stop() => done += 1,
                None => {}
            }
            if sh.stop() {
                break;
            }
        }
        let (msg, kind) = if sh.stop() {
            (i18n::t(Msg::Cancelled).to_string(), Kind::Warn)
        } else if !errors.is_empty() {
            (tf(Msg::PartialErr, &[&done, &total, &errors.join(" ; ")]), Kind::Err)
        } else {
            let at = Local::now().format("%H:%M:%S%.3f").to_string();
            (if total > 1 { tf(Msg::DoneMany, &[&at, &done]) } else { tf(Msg::Done, &[&at]) }, Kind::Ok)
        };
        engine::keep_awake(false);
        sh.running.store(false, Ordering::SeqCst);
        // si la dernière action a rendu la main à l'utilisateur (« revenir où j'étais »), on ne lui vole pas le focus
        let gives_back = jobs.last().is_some_and(|j| j.back && j.target.is_some());
        if minimized && !gives_back {
            sh.restore.store(true, Ordering::SeqCst);
        }
        sh.set_status(&ctx, msg, kind);
    });
}

/// Repère la zone à cibler : affiche une bannière, attend un clic (Échap = annuler) et décrit la fenêtre cliquée.
pub fn spawn_pick(sh: Arc<Shared>, ctx: egui::Context) {
    sh.picking.store(true, Ordering::SeqCst);
    *sh.picked.lock().unwrap() = None;
    std::thread::spawn(move || {
        let banner = engine::Banner::show(i18n::t(Msg::PickBanner));
        let start = Instant::now();
        let own = engine::own_pid();
        let result = loop {
            sleep(Duration::from_millis(15));
            if start.elapsed() < Duration::from_millis(500) {
                continue; // laisse le temps de relâcher le clic du bouton
            }
            if engine::escape_down() {
                break None;
            }
            if engine::left_button_down() {
                let (x, y) = engine::cursor_pos();
                if let Some((t, h)) = engine::target_at(x, y) {
                    if engine::win_pid(h) != own {
                        break Some(t);
                    }
                }
            }
        };
        drop(banner);
        sh.picking.store(false, Ordering::SeqCst);
        *sh.picked.lock().unwrap() = Some(result);
        ctx.request_repaint();
    });
}
