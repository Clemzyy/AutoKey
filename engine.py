"""AutoKey - moteur Windows : envoi de touches, fenêtres, souris, anti-veille (aucune interface ici)."""
import ctypes
import os
import time
from ctypes import wintypes

# ---------- Envoi de touches (SendInput, par code de balayage = touche physique) ----------
KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE = 0x1, 0x2, 0x8
ULONG_PTR = ctypes.c_size_t


class KEYBDINPUT(ctypes.Structure):
    _fields_ = [("wVk", wintypes.WORD), ("wScan", wintypes.WORD), ("dwFlags", wintypes.DWORD),
                ("time", wintypes.DWORD), ("dwExtraInfo", ULONG_PTR)]


class MOUSEINPUT(ctypes.Structure):
    _fields_ = [("dx", wintypes.LONG), ("dy", wintypes.LONG), ("mouseData", wintypes.DWORD),
                ("dwFlags", wintypes.DWORD), ("time", wintypes.DWORD), ("dwExtraInfo", ULONG_PTR)]


class INPUT(ctypes.Structure):
    class _U(ctypes.Union):
        _fields_ = [("ki", KEYBDINPUT), ("mi", MOUSEINPUT)]
    _anonymous_ = ("u",)
    _fields_ = [("type", wintypes.DWORD), ("u", _U)]


def send_scan(scan, ext, up):
    flags = KEYEVENTF_SCANCODE | (KEYEVENTF_EXTENDEDKEY if ext else 0) | (KEYEVENTF_KEYUP if up else 0)
    inp = INPUT(type=1, ki=KEYBDINPUT(0, scan, flags, 0, 0))
    ctypes.windll.user32.SendInput(1, ctypes.byref(inp), ctypes.sizeof(INPUT))


def send_char(ch):
    for up in (False, True):
        flags = 0x4 | (KEYEVENTF_KEYUP if up else 0)  # KEYEVENTF_UNICODE
        inp = INPUT(type=1, ki=KEYBDINPUT(0, ord(ch), flags, 0, 0))
        ctypes.windll.user32.SendInput(1, ctypes.byref(inp), ctypes.sizeof(INPUT))


# ---------- Fenêtres et souris ----------
try:
    ctypes.windll.shcore.SetProcessDpiAwareness(2)   # coordonnées en pixels réels
except Exception:
    try:
        ctypes.windll.user32.SetProcessDPIAware()
    except Exception:
        pass

user32, kernel32 = ctypes.windll.user32, ctypes.windll.kernel32
HWND = ctypes.c_void_p
for _f, _res, _args in (
        ("GetForegroundWindow", HWND, []), ("SetForegroundWindow", wintypes.BOOL, [HWND]),
        ("BringWindowToTop", wintypes.BOOL, [HWND]), ("IsWindow", wintypes.BOOL, [HWND]),
        ("IsWindowVisible", wintypes.BOOL, [HWND]), ("IsIconic", wintypes.BOOL, [HWND]),
        ("ShowWindow", wintypes.BOOL, [HWND, ctypes.c_int]),
        ("GetWindowRect", wintypes.BOOL, [HWND, ctypes.POINTER(wintypes.RECT)]),
        ("GetWindowTextW", ctypes.c_int, [HWND, wintypes.LPWSTR, ctypes.c_int]),
        ("GetClassNameW", ctypes.c_int, [HWND, wintypes.LPWSTR, ctypes.c_int]),
        ("GetWindowThreadProcessId", wintypes.DWORD, [HWND, ctypes.POINTER(wintypes.DWORD)]),
        ("WindowFromPoint", HWND, [wintypes.POINT]), ("GetAncestor", HWND, [HWND, wintypes.UINT]),
        ("AttachThreadInput", wintypes.BOOL, [wintypes.DWORD, wintypes.DWORD, wintypes.BOOL]),
        ("EnumWindows", wintypes.BOOL, [ctypes.c_void_p, wintypes.LPARAM])):
    getattr(user32, _f).restype, getattr(user32, _f).argtypes = _res, _args
kernel32.OpenProcess.restype = ctypes.c_void_p
kernel32.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
kernel32.QueryFullProcessImageNameW.argtypes = [ctypes.c_void_p, wintypes.DWORD, wintypes.LPWSTR,
                                               ctypes.POINTER(wintypes.DWORD)]
kernel32.CloseHandle.argtypes = [ctypes.c_void_p]
kernel32.SetThreadExecutionState.argtypes = [wintypes.DWORD]
WNDENUMPROC = ctypes.WINFUNCTYPE(wintypes.BOOL, HWND, wintypes.LPARAM)
user32.EnumWindows.argtypes = [WNDENUMPROC, wintypes.LPARAM]
VK_LBUTTON, VK_ESCAPE, GA_ROOT = 0x01, 0x1B, 2
ES_CONTINUOUS, ES_SYSTEM_REQUIRED, ES_DISPLAY_REQUIRED = 0x80000000, 0x1, 0x2


def keep_awake(on):
    """Empêche la veille (et l'extinction de l'écran) tant qu'une action est armée."""
    flags = ES_CONTINUOUS | ((ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED) if on else 0)
    kernel32.SetThreadExecutionState(flags)


def panic_pressed():
    """Arrêt d'urgence : Ctrl + Alt + Échap, même si la fenêtre est réduite."""
    return all(user32.GetAsyncKeyState(vk) & 0x8000 for vk in (0x11, 0x12, VK_ESCAPE))


def strip_hwnd(t):
    return {k: v for k, v in t.items() if k != "hwnd"} if t else None


def win_title(h):
    buf = ctypes.create_unicode_buffer(512)
    user32.GetWindowTextW(h, buf, 512)
    return buf.value


def win_class(h):
    buf = ctypes.create_unicode_buffer(256)
    user32.GetClassNameW(h, buf, 256)
    return buf.value


def win_exe(h):
    pid = wintypes.DWORD()
    user32.GetWindowThreadProcessId(h, ctypes.byref(pid))
    ph = kernel32.OpenProcess(0x1000, False, pid.value)   # PROCESS_QUERY_LIMITED_INFORMATION
    if not ph:
        return ""
    try:
        buf, n = ctypes.create_unicode_buffer(1024), wintypes.DWORD(1024)
        kernel32.QueryFullProcessImageNameW(ph, 0, buf, ctypes.byref(n))
        return os.path.basename(buf.value).lower()
    finally:
        kernel32.CloseHandle(ph)


def win_rect(h):
    r = wintypes.RECT()
    user32.GetWindowRect(h, ctypes.byref(r))
    return r.left, r.top, r.right, r.bottom


def root_at(x, y):
    h = user32.WindowFromPoint(wintypes.POINT(x, y))
    return user32.GetAncestor(h, GA_ROOT) if h else None


def cursor_pos():
    pt = wintypes.POINT()
    user32.GetCursorPos(ctypes.byref(pt))
    return pt.x, pt.y


def mouse_click(x, y):
    user32.SetCursorPos(x, y)
    time.sleep(0.05)
    for flag in (0x2, 0x4):     # bouton gauche enfoncé / relâché
        inp = INPUT(type=0, mi=MOUSEINPUT(0, 0, 0, flag, 0, 0))
        user32.SendInput(1, ctypes.byref(inp), ctypes.sizeof(INPUT))
        time.sleep(0.03)


def find_window(t):
    """Retrouve la fenêtre cible : même handle si elle existe encore, sinon même programme + même classe."""
    h = t.get("hwnd")
    if h and user32.IsWindow(h) and win_exe(h) == t["exe"] and win_class(h) == t["cls"]:
        return h
    found = []

    @WNDENUMPROC
    def cb(hw, _):
        if user32.IsWindowVisible(hw) and win_exe(hw) == t["exe"] and win_class(hw) == t["cls"]:
            found.append(hw)
        return True
    user32.EnumWindows(cb, 0)
    for hw in found:                      # titre identique d'abord, sinon le premier
        if win_title(hw) == t["title"]:
            return hw
    return found[0] if found else None


def focus_window(h):
    """Amène la fenêtre au premier plan (la restaure si réduite). True si réussi."""
    if user32.IsIconic(h):
        user32.ShowWindow(h, 9)           # SW_RESTORE
    for attempt in range(2):
        fg = user32.GetForegroundWindow()
        if fg == h:
            return True
        cur = kernel32.GetCurrentThreadId()
        fg_thread = user32.GetWindowThreadProcessId(fg, None) if fg else 0
        if attempt:                       # 2e essai : touche Alt "neutre" pour lever le blocage de Windows
            user32.keybd_event(0x12, 0, 0, 0)
            user32.keybd_event(0x12, 0, 2, 0)
        if fg_thread and fg_thread != cur:
            user32.AttachThreadInput(cur, fg_thread, True)
        user32.BringWindowToTop(h)
        user32.SetForegroundWindow(h)
        if fg_thread and fg_thread != cur:
            user32.AttachThreadInput(cur, fg_thread, False)
        for _ in range(20):
            time.sleep(0.05)
            if user32.GetForegroundWindow() == h:
                return True
    return False


def click_target(t):
    """Focus la fenêtre cible puis clique dans la zone repérée. Lève RuntimeError avec la raison si impossible."""
    h = find_window(t)
    if not h:
        raise RuntimeError("fenêtre cible introuvable")
    if not focus_window(h):
        raise RuntimeError("impossible de mettre la fenêtre au premier plan")
    l, tp, r, b = win_rect(h)
    w, hgt = r - l, b - tp
    if (w, hgt) == (t["w"], t["h"]):
        x, y = l + t["dx"], tp + t["dy"]
    else:                                  # fenêtre redimensionnée : position proportionnelle
        x, y = l + int(t["fx"] * w), tp + int(t["fy"] * hgt)
    if root_at(x, y) != h:
        raise RuntimeError("la zone cible est masquée par une autre fenêtre")
    mouse_click(x, y)
    time.sleep(0.15)
    return h


# ---------- Disposition AZERTY : (étiquette, code de balayage, extended, largeur) ----------
def k(label, scan, w=1, ext=False):
    return (label, scan, ext, w)

ROWS = [
    [k("Échap", 0x01, 1.5)] + [k(f"F{i}", s) for i, s in
        zip(range(1, 11), range(0x3B, 0x45))] + [k("F11", 0x57), k("F12", 0x58)],
    [k("²", 0x29)] + [k(c, s) for c, s in zip("&é\"'(-è_çà)=", range(0x02, 0x0E))] + [k("⌫", 0x0E, 2)],
    [k("Tab", 0x0F, 1.5)] + [k(c, s) for c, s in zip("AZERTYUIOP", range(0x10, 0x1A))]
        + [k("^", 0x1A), k("$", 0x1B), k("Entrée", 0x1C, 1.5)],
    [k("Verr.Maj", 0x3A, 1.8)] + [k(c, s) for c, s in zip("QSDFGHJKLM", range(0x1E, 0x28))]
        + [k("ù", 0x28), k("*", 0x2B)],
    [k("Maj", 0x2A, 1.3), k("<", 0x56)] + [k(c, s) for c, s in zip("WXCVBN", range(0x2C, 0x32))]
        + [k(",", 0x32), k(";", 0x33), k(":", 0x34), k("!", 0x35), k("Maj ", 0x36, 2.2)],
    [k("Ctrl", 0x1D, 1.5), k("Alt", 0x38, 1.5), k("Espace", 0x39, 7), k("Alt Gr", 0x38, 1.5, True),
     k("←", 0x4B, 1, True), k("↑", 0x48, 1, True), k("↓", 0x50, 1, True), k("→", 0x4D, 1, True)],
]
# Modificateurs maintenus pendant les autres touches (clé = étiquette)
MODS = {"Maj": (0x2A, False), "Maj ": (0x36, False), "Ctrl": (0x1D, False),
        "Alt": (0x38, False), "Alt Gr": (0x38, True)}

LOOKUP = {l.strip().lower(): (sc, e) for row in ROWS for l, sc, e, _ in row if l not in MODS}
LOOKUP.update({"enter": LOOKUP["entrée"], "esc": LOOKUP["échap"], "backspace": LOOKUP["⌫"],
               "space": LOOKUP["espace"]})
