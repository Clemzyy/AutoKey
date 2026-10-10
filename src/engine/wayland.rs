//! Envoi de touches sous Wayland, par le portail « RemoteDesktop » (xdg-desktop-portal).
//!
//! Wayland interdit à un programme d'envoyer des touches à une autre fenêtre ; le portail est la voie officielle :
//! le système affiche une fenêtre de confirmation, et l'utilisateur autorise AutoKey. Quand le bureau le permet,
//! l'accord est mémorisé (jeton) et n'est plus redemandé. Seul le clavier est demandé, jamais la souris ni l'écran.
//!
//! Le portail ne permet ni de lister ni de choisir les fenêtres : les touches vont toujours dans la fenêtre active.
use std::sync::mpsc::{sync_channel, SyncSender};
use std::sync::Mutex;
use std::time::Duration;

use ashpd::desktop::remote_desktop::{DeviceType, KeyState, RemoteDesktop, SelectDevicesOptions};
use ashpd::desktop::PersistMode;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};

/// Où en est l'autorisation.
#[derive(Clone, Debug, PartialEq)]
pub enum State {
    /// Pas encore demandée.
    Needed,
    /// Fenêtre de confirmation affichée, en attente de la réponse.
    Pending,
    /// Accordée : les touches peuvent partir.
    Granted,
    /// Refusée (ou session fermée) ; le texte décrit la cause.
    Denied(String),
    /// Le bureau n'offre pas ce portail.
    Unavailable,
}

enum Cmd {
    Keycode(i32, bool, SyncSender<bool>),
}

static STATE: Mutex<State> = Mutex::new(State::Needed);
static TX: Mutex<Option<UnboundedSender<Cmd>>> = Mutex::new(None);

pub fn state() -> State {
    STATE.lock().map(|s| s.clone()).unwrap_or(State::Needed)
}

fn set_state(s: State) {
    if let Ok(mut st) = STATE.lock() {
        *st = s;
    }
}

/// Fichier où est gardé le jeton qui rend l'autorisation permanente.
fn token_path() -> std::path::PathBuf {
    crate::model::config_dir().join("AutoKey").join("wayland-token")
}

/// Vrai si un accord mémorisé existe : l'autorisation peut alors être rétablie sans fenêtre de confirmation.
pub fn has_saved_token() -> bool {
    std::fs::read_to_string(token_path()).is_ok_and(|t| !t.trim().is_empty())
}

/// Demande l'autorisation (sans effet si elle est déjà accordée ou en cours). Ne bloque pas : la réponse de
/// l'utilisateur est lue via `state()`.
pub fn request() {
    {
        let Ok(mut st) = STATE.lock() else { return };
        if matches!(*st, State::Pending | State::Granted | State::Unavailable) {
            return;
        }
        *st = State::Pending;
    }
    std::thread::spawn(|| {
        let runtime = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
            Ok(r) => r,
            Err(e) => return set_state(State::Denied(e.to_string())),
        };
        let outcome = runtime.block_on(session());
        if let Ok(mut tx) = TX.lock() {
            *tx = None;
        }
        // une session terminée normalement (fermée par le système) se redemande au prochain besoin
        set_state(match outcome {
            Ok(()) => State::Needed,
            Err(state) => state,
        });
    });
}

/// Ouvre la session, attend l'accord, puis exécute les envois de touches jusqu'à la fermeture de la session.
async fn session() -> Result<(), State> {
    let denied = |e: ashpd::Error| State::Denied(e.to_string());
    let proxy = RemoteDesktop::new().await.map_err(|_| State::Unavailable)?;
    let session = proxy.create_session(Default::default()).await.map_err(|_| State::Unavailable)?;
    let saved = std::fs::read_to_string(token_path()).ok().map(|t| t.trim().to_string()).filter(|t| !t.is_empty());
    proxy
        .select_devices(
            &session,
            SelectDevicesOptions::default()
                .set_devices(ashpd::enumflags2::BitFlags::from_flag(DeviceType::Keyboard))
                .set_persist_mode(PersistMode::ExplicitlyRevoked)
                .set_restore_token(saved.as_deref()),
        )
        .await
        .map_err(denied)?;
    let response = proxy.start(&session, None, Default::default()).await.map_err(denied)?.response().map_err(denied)?;
    if !response.devices().contains(DeviceType::Keyboard) {
        return Err(State::Denied(String::new()));
    }
    if let Some(token) = response.restore_token() {
        let path = token_path();
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(path, token);
    }
    let (tx, mut rx) = unbounded_channel::<Cmd>();
    if let Ok(mut slot) = TX.lock() {
        *slot = Some(tx);
    }
    set_state(State::Granted);
    let key = |down: bool| if down { KeyState::Pressed } else { KeyState::Released };
    while let Some(cmd) = rx.recv().await {
        let (ok, reply) = match cmd {
            Cmd::Keycode(code, down, reply) => (proxy.notify_keyboard_keycode(&session, code, key(down), Default::default()).await.is_ok(), reply),
        };
        let _ = reply.send(ok);
        if !ok {
            return Err(State::Denied(String::new())); // session fermée par le système : à redemander
        }
    }
    Ok(())
}

fn send(make: impl FnOnce(SyncSender<bool>) -> Cmd) -> bool {
    let (reply, wait) = sync_channel(1);
    let sent = TX.lock().ok().and_then(|tx| tx.as_ref().map(|tx| tx.send(make(reply)).is_ok())).unwrap_or(false);
    sent && wait.recv_timeout(Duration::from_secs(3)).unwrap_or(false)
}

/// Appuie / relâche une touche par son code évdev (celui des claviers Linux).
pub fn keycode(code: i32, down: bool) -> bool {
    send(|reply| Cmd::Keycode(code, down, reply))
}
