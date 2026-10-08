"""Test fonctionnel de l'interface d'AutoKey (Rust) : vraie souris, vrai clavier, état lu via AUTOKEY_STATE.

Usage : python tests/ui_test.py chemin/vers/autokey.exe   (Windows ; utilise ses propres réglages, pas les tiens)
"""
import ctypes, json, os, subprocess, sys, tempfile, time
from ctypes import wintypes as w



class _KI(ctypes.Structure):
    _fields_ = [("wVk", w.WORD), ("wScan", w.WORD), ("dwFlags", w.DWORD), ("time", w.DWORD), ("extra", ctypes.c_size_t)]


class _MI(ctypes.Structure):
    _fields_ = [("dx", w.LONG), ("dy", w.LONG), ("data", w.DWORD), ("flags", w.DWORD), ("time", w.DWORD), ("extra", ctypes.c_size_t)]


class _U(ctypes.Union):
    _fields_ = [("ki", _KI), ("mi", _MI)]


class _IN(ctypes.Structure):
    _fields_ = [("type", w.DWORD), ("u", _U)]


class _Eng:
    """Envoi d'un caractère Unicode au clavier (SendInput), sans dépendance externe."""

    @staticmethod
    def send_char(ch):
        for up in (0, 2):                       # 4 = KEYEVENTF_UNICODE, 2 = KEYUP
            inp = _IN(1, _U(ki=_KI(0, ord(ch), 4 | up, 0, 0)))
            ctypes.windll.user32.SendInput(1, ctypes.byref(inp), ctypes.sizeof(inp))


eng = _Eng()

u = ctypes.windll.user32
u.SetProcessDPIAware()
u.FindWindowW.restype = ctypes.c_void_p
u.GetForegroundWindow.restype = ctypes.c_void_p
EXE = sys.argv[1]
TMP = tempfile.mkdtemp(prefix="akui_")
SETTINGS = os.path.join(TMP, "settings.json")
STATE = os.path.join(TMP, "state.json")
SHOTS = os.path.join(TMP, "shots"); os.makedirs(SHOTS)
TITLE = "AutoKey – touche à heure précise"
results = []


def check(name, ok, detail=""):
    results.append((name, bool(ok), detail))
    print(("  OK    " if ok else "  ÉCHEC ") + name + ("" if ok else f"   -> {detail}"), flush=True)


def start(preset=None):
    for f in (STATE, SETTINGS):
        if os.path.exists(f): os.remove(f)
    if preset is not None:
        json.dump(preset, open(SETTINGS, "w", encoding="utf-8"))
    env = dict(os.environ, AUTOKEY_SETTINGS=SETTINGS, AUTOKEY_STATE=STATE, AUTOKEY_SHOTS=SHOTS)
    p = subprocess.Popen([EXE], env=env)
    for _ in range(80):
        if os.path.exists(STATE): break
        time.sleep(0.1)
    time.sleep(1.5)
    return p


def hwnd():
    return ctypes.c_void_p(u.FindWindowW(None, TITLE))


def focus():
    h = hwnd(); u.ShowWindow(h, 9); u.SetForegroundWindow(h); time.sleep(0.25)
    return u.GetForegroundWindow() == h.value


def state():
    for _ in range(20):
        try:
            return json.load(open(STATE, encoding="utf-8"))
        except Exception:
            time.sleep(0.05)
    raise RuntimeError("état illisible")


def wait(pred, timeout=2.0):
    end = time.time() + timeout
    while time.time() < end:
        try:
            if pred(state()): return True
        except Exception: pass
        time.sleep(0.06)
    return False


def origin():
    pt = w.POINT(0, 0); u.ClientToScreen(hwnd(), ctypes.byref(pt)); return pt.x, pt.y


def find(name, contains=False):
    ws = state()["widgets"]
    if not contains and name in ws: return ws[name]
    for k, v in ws.items():
        if name in k: return v
    raise KeyError(f"composant introuvable : {name}")


def pos(name, rx=0.5, ry=0.5):
    x, y, ww, hh = find(name, contains=True)
    ox, oy = origin()
    return int(ox + x + ww * rx), int(oy + y + hh * ry)


def mouse(x, y, dbl=False):
    u.SetCursorPos(x, y); time.sleep(0.08)
    for _ in range(2 if dbl else 1):
        u.mouse_event(2, 0, 0, 0, 0); time.sleep(0.04); u.mouse_event(4, 0, 0, 0, 0); time.sleep(0.06)
    time.sleep(0.25)


def click(name, dbl=False, rx=0.5, ry=0.5):
    focus()
    mouse(*pos(name, rx, ry), dbl=dbl)


def key(vk, ctrl=False):
    if ctrl: u.keybd_event(0x11, 0, 0, 0)
    u.keybd_event(vk, 0, 0, 0); time.sleep(0.03); u.keybd_event(vk, 0, 2, 0)
    if ctrl: u.keybd_event(0x11, 0, 2, 0)
    time.sleep(0.2)


def typ(text):
    assert focus(), "AutoKey n'a pas le focus : frappe annulée"
    for c in text:
        eng.send_char(c); time.sleep(0.03)
    time.sleep(0.25)


def wheel(name, delta=120, ctrl=False):
    focus(); x, y = pos(name); u.SetCursorPos(x, y); time.sleep(0.15)
    if ctrl: u.keybd_event(0x11, 0, 0, 0)
    u.mouse_event(0x800, 0, 0, delta, 0); time.sleep(0.15)
    if ctrl: u.keybd_event(0x11, 0, 2, 0)
    time.sleep(0.3)


def shot(name):
    open(os.path.join(SHOTS, "req.txt"), "w").write(name)
    end = time.time() + 4
    while time.time() < end and not os.path.exists(os.path.join(SHOTS, name + ".png")): time.sleep(0.1)
    return os.path.join(SHOTS, name + ".png")


def saved():
    return json.load(open(SETTINGS, encoding="utf-8"))


# ============================ SÉRIE 1 : réglages vierges ============================
print("== Série 1 : interface complète (réglages vierges)")
app = start()
s = state()
check("démarrage : état initial", s["text"] == "" and not s["list_mode"] and s["status"].startswith("Arrêt d'urgence"), s["status"])
focus()

click("touche:A"); check("clavier : clic sur A -> [A]", wait(lambda s: s["text"] == "[A]"), state()["text"])
click("touche:Entrée"); check("clavier : clic sur Entrée -> [A][Entrée]", wait(lambda s: s["text"] == "[A][Entrée]"), state()["text"])
click("touche:Ctrl"); check("clavier : Ctrl maintenu", wait(lambda s: s["mods"] == ["Ctrl"]), state()["mods"])
click("touche:Ctrl"); check("clavier : Ctrl relâché", wait(lambda s: s["mods"] == []), state()["mods"])

x, y = pos("texte", 0.97, 0.5); mouse(x, y); typ("xy"); key(0x25)   # curseur entre x et y
check("texte : frappe au clavier", wait(lambda s: s["text"] == "[A][Entrée]xy"), state()["text"])
click("touche:Q"); check("texte : insertion à la position du curseur", wait(lambda s: s["text"] == "[A][Entrée]x[Q]y"), state()["text"])

click("Effacer"); check("bouton Effacer", wait(lambda s: s["text"] == "" and s["mods"] == []), state()["text"])

click("Date"); check("interrupteur Date activé", wait(lambda s: s["use_date"]))
mouse(*pos("date")); key(0x41, ctrl=True); typ("99/99/9999")
check("champ date : saisie", wait(lambda s: s["date"] == "99/99/9999"), state()["date"])
click("Date"); check("interrupteur Date désactivé", wait(lambda s: not s["use_date"]))

click("Répéter"); check("interrupteur Répéter activé", wait(lambda s: s["repeat"]))
rep0 = state()["rep"]
wheel("num:rep", 120, ctrl=False); check("molette sans Ctrl : valeur inchangée", state()["rep"] == rep0, state()["rep"])
wheel("num:rep", 120, ctrl=True); check("Ctrl + molette : valeur +1", wait(lambda s: s["rep"] == rep0 + 1), state()["rep"])
click("Répéter"); check("interrupteur Répéter désactivé", wait(lambda s: not s["repeat"]))

# heure : double-clic dans le champ, saisie, Entrée
focus(); mouse(*pos("num:h"), dbl=True); key(0x41, ctrl=True); typ("07"); key(0x0D)
check("champ heure : saisie de 07", wait(lambda s: s["h"] == 7), state()["h"])

# --- menu Options ---
click("Options"); check("Options : menu ouvert", wait(lambda s: s["opt_open"]))
check("Options : trois entrées visibles", all(any(t in k for k in state()["widgets"]) for t in ("Réduire", "Aller dans", "Revenir")))
shot("options")
click("Réduire"); check("Options : « Réduire » désactivé", wait(lambda s: not s["minim"]))
click("Aller dans"); check("Options : « Aller dans » désactivé", wait(lambda s: not s["use_target"]))
click("Revenir"); check("Options : « Revenir » désactivé", wait(lambda s: not s["back"]))
click("Réduire"); click("Aller dans"); click("Revenir")
check("Options : les trois réactivés", wait(lambda s: s["minim"] and s["use_target"] and s["back"]))
ox, oy = origin(); mouse(ox + 400, oy + 50)
check("Options : clic à côté ferme le menu", wait(lambda s: not s["opt_open"]))

# --- validations à l'armement ---
click("Armer"); check("Armer sans texte : avertissement", wait(lambda s: "Rien à envoyer" in s["status"] and s["kind"] == "Warn"), state()["status"])
mouse(*pos("texte", 0.97, 0.5)); typ("a")
click("Date"); mouse(*pos("date")); key(0x41, ctrl=True); typ("99/99/9999")
click("Armer"); check("Armer avec date invalide : erreur", wait(lambda s: "invalide" in s["status"] and s["kind"] == "Err"), state()["status"])
mouse(*pos("date")); key(0x41, ctrl=True); typ("01/01/2020")
click("Armer"); check("Armer avec date passée : erreur", wait(lambda s: "passée" in s["status"] and s["kind"] == "Err"), state()["status"])
click("Date"); wait(lambda s: not s["use_date"])
mouse(*pos("texte", 0.97, 0.5)); typ("[Toto]")
click("Armer"); check("Armer avec touche inconnue : erreur", wait(lambda s: "Touche inconnue" in s["status"] and s["kind"] == "Err"), state()["status"])
click("Effacer"); mouse(*pos("texte", 0.97, 0.5)); typ("a")

# --- armer puis annuler ---
click("Armer"); check("Armer : l'envoi démarre (en attente)", wait(lambda s: s["running"] and " dans " in s["status"]), state()["status"])
click("Annuler"); check("Annuler : arrêt immédiat", wait(lambda s: not s["running"] and "Annulé" in s["status"], 3), state()["status"])

# --- mode liste ---
click("Mode liste d'actions"); check("mode liste : activé", wait(lambda s: s["list_mode"]))
time.sleep(0.8)
click("Ajouter l'action"); check("liste : ajout d'une action", wait(lambda s: s["actions"] == ["a"]), state()["actions"])
check("liste : sauvegardée tout de suite sur disque", [a["text"] for a in saved()["actions"]] == ["a"], saved()["actions"])
click("Effacer"); mouse(*pos("texte", 0.97, 0.5)); typ("b"); click("Ajouter l'action")
check("liste : deuxième action", wait(lambda s: s["actions"] == ["a", "b"]), state()["actions"])
shot("liste")
click("ligne0"); check("liste : sélection d'une ligne", wait(lambda s: s["sel"] == 0), state()["sel"])
click("ligne0", dbl=True); check("liste : double-clic recharge l'action", wait(lambda s: s["text"] == "a"), state()["text"])
click("ligne1"); click("Supprimer"); check("liste : suppression", wait(lambda s: s["actions"] == ["a"]), state()["actions"])
click("Remplacer"); check("liste : remplacer sans sélection -> message", wait(lambda s: "Sélectionne" in s["status"]), state()["status"])
click("Tout vider"); check("liste : tout vider", wait(lambda s: s["actions"] == []), state()["actions"])
check("liste : vidée aussi sur disque", saved()["actions"] == [], saved()["actions"])
click("Armer la liste"); check("liste vide : avertissement", wait(lambda s: "liste est vide" in s["status"]), state()["status"])
click("Mode liste d'actions"); check("mode liste : désactivé", wait(lambda s: not s["list_mode"]))

# --- fenêtre étroite / redimensionnement ---
h = hwnd(); r = w.RECT(); u.GetWindowRect(h, ctypes.byref(r)); ppp = state()["ppp"]
u.SetWindowPos(h, None, r.left, r.top, int(1010 * ppp), int(700 * ppp), 0x0004 | 0x0010); time.sleep(0.8)
shot("etroit"); check("redimensionnement à la largeur minimale : l'application répond", wait(lambda s: s["ppp"] > 0))
u.SetWindowPos(h, None, r.left, r.top, int(1300 * ppp), int(900 * ppp), 0x0004 | 0x0010); time.sleep(0.5)
check("redimensionnement agrandi : l'application répond", wait(lambda s: s["ppp"] > 0))

# --- fermeture pendant l'attente : processus terminé et réglages sauvegardés ---
click("Armer"); wait(lambda s: s["running"])
u.PostMessageW(hwnd(), 0x0010, 0, 0)       # WM_CLOSE
t0 = time.time()
while app.poll() is None and time.time() - t0 < 5: time.sleep(0.1)
check("fermeture pendant l'attente : le processus se termine", app.poll() is not None, f"{time.time()-t0:.1f}s")
check("fermeture : réglages sauvegardés", os.path.exists(SETTINGS) and saved().get("text") == "a", saved().get("text") if os.path.exists(SETTINGS) else "pas de fichier")
if app.poll() is None: app.kill()

# ============================ SÉRIE 2 : Test (3 s) dans une vraie fenêtre ============================
print("== Série 2 : bouton Test dans une fenêtre cible")
pos_file = os.path.join(TMP, "child.pos"); out_file = os.path.join(TMP, "child.out")
child_src = r'''
import ctypes; ctypes.windll.shcore.SetProcessDpiAwareness(2)
import tkinter as tk
r = tk.Tk(); r.title("CIBLE_UI"); r.geometry("500x240+300+300")
e = tk.Entry(r, width=36, font=("Consolas", 14)); e.pack(pady=40)
r.after(1500, lambda: open(%r, "w").write(f"{e.winfo_rootx()+30},{e.winfo_rooty()+10}"))
r.after(22000, lambda: (open(%r, "w").write(e.get()), r.destroy()))
r.mainloop()
''' % (pos_file, out_file)
child = subprocess.Popen([sys.executable, "-c", child_src]); time.sleep(3)
ch = ctypes.c_void_p(u.FindWindowW(None, "CIBLE_UI")); rc = w.RECT(); u.GetWindowRect(ch, ctypes.byref(rc))
cx, cy = map(int, open(pos_file).read().split(","))
target = dict(exe=os.path.basename(sys.executable).lower(), cls="TkTopLevel", title="CIBLE_UI", dx=cx - rc.left, dy=cy - rc.top,
              w=rc.right - rc.left, h=rc.bottom - rc.top, fx=(cx - rc.left) / (rc.right - rc.left), fy=(cy - rc.top) / (rc.bottom - rc.top))
preset = {"text": "t1[F12]", "mods": [], "repeat": False, "rep": "10", "gap": "100", "minim": False, "use_date": False, "date": "01/01/2030",
          "use_target": True, "back": True, "target": target, "list_mode": False, "actions": []}
app = start(preset)
s = state(); check("réglages chargés (cible mémorisée)", s["target"] == "python.exe" and s["text"] == "t1[F12]", s["target"])
click("Test (3 s)")
check("Test : l'envoi démarre", wait(lambda s: s["running"], 3))
check("Test : terminé avec succès", wait(lambda s: not s["running"] and s["status"].startswith("✔"), 12), state()["status"])
if app.poll() is None: app.terminate()
child.wait(timeout=40)
got = open(out_file, encoding="cp1252").read()
check("Test : le texte est arrivé dans la fenêtre cible", got == "t1", repr(got))

# ============================ bilan ============================
ok = sum(1 for _, o, _ in results if o)
print(f"\nRÉSULTAT : {ok}/{len(results)} vérifications réussies")
for n, o, d in results:
    if not o: print("  À REGARDER :", n, "|", d)
print("captures :", SHOTS)
json.dump({"shots": SHOTS, "tmp": TMP, "ok": ok, "total": len(results)}, open(os.path.join(tempfile.gettempdir(), "ui_test_last.json"), "w"))
