"""AutoKey - tape des touches ou du texte à une heure précise, dans la fenêtre de ton choix (Windows)."""
import ctypes
import json
import os
import re
import sys
import threading
import time
import tkinter as tk
import tkinter.font as tkfont
import webbrowser
from datetime import datetime, timedelta
from tkinter import messagebox

import customtkinter as ctk
from PIL import Image

from engine import (GA_ROOT, LOOKUP, MODS, ROWS, VK_ESCAPE, VK_LBUTTON, click_target, cursor_pos, focus_window,
                    keep_awake, panic_pressed, root_at, send_char, send_scan, strip_hwnd, user32, win_class,
                    win_exe, win_rect, win_title)

APP_NAME = "AutoKey"
AUTHOR = "Clemzy aka InforMagicien"
DONATE_URL = "https://www.paypal.com/donate/?hosted_button_id=NKCR6KK739WGS"
SETTINGS = os.path.join(os.environ.get("APPDATA", os.path.expanduser("~")), APP_NAME, "reglages.json")

# ---------- thème ----------
BG, CARD, FIELD, BORDER = "#0d0f17", "#161a26", "#1e2333", "#2a3047"
KEY, KEY_HOV = "#242a3d", "#343d5c"
TXT, MUTED = "#e9ecf8", "#8a91ad"
ACCENT, ACCENT_HOV = "#7c5cff", "#9378ff"
GREEN, GREEN_HOV, RED, RED_HOV = "#22c55e", "#34d474", "#ef4444", "#f66464"
VIOLET, JAUNE, ORANGE, GRIS = "#c9b1ff", "#f5d63d", "#f08a24", "#3a4058"
ROW, ROW_SEL = "#1c2132", "#2f2a5c"
FONT = "Segoe UI"


def asset(name):
    base = getattr(sys, "_MEIPASS", os.path.dirname(os.path.abspath(__file__)))
    return os.path.join(base, "assets", name)


def font(size=13, weight="normal", family=FONT):
    return ctk.CTkFont(family=family, size=size, weight=weight)


def num_entry(parent, var, lo, hi, width=64, digits=2, wrap=True, step=1, size=18):
    """Champ numérique : flèches ↑↓ et molette pour changer la valeur."""
    e = ctk.CTkEntry(parent, textvariable=var, width=width, height=38, justify="center", corner_radius=10,
                     font=font(size, "bold", "Consolas"), fg_color=FIELD, border_color=BORDER, text_color=TXT)

    def clamp(v):
        return (v - lo) % (hi - lo + 1) + lo if wrap else max(lo, min(hi, v))

    def bump(d):
        try:
            v = int(var.get())
        except ValueError:
            v = lo
        var.set(str(clamp(v + d * step)).zfill(digits))
        return "break"

    def norm(_=None):
        try:
            var.set(str(clamp(int(var.get()))).zfill(digits))
        except ValueError:
            var.set(str(lo).zfill(digits))
    e.bind("<Up>", lambda ev: bump(1))
    e.bind("<Down>", lambda ev: bump(-1))
    e.bind("<MouseWheel>", lambda ev: bump(1 if ev.delta > 0 else -1))
    e.bind("<FocusOut>", norm)
    return e


class KeyboardCanvas(tk.Canvas):
    """Clavier AZERTY dessiné dans un seul Canvas : redimensionnement fluide (un seul widget)."""
    KEY_H, GAP = 30, 3          # hauteur d'une touche et espace entre touches, en unités indépendantes de l'écran

    def __init__(self, parent, scale, on_key):
        self.sc = scale
        self.rows_n = len(ROWS)
        super().__init__(parent, bg=CARD, highlightthickness=0, bd=0, cursor="hand2",
                         height=int(self.rows_n * (self.KEY_H + self.GAP) * scale))
        self.on_key = on_key
        self.fnt = tkfont.Font(family=FONT, size=-int(round(12 * scale * 1.33)), weight="bold")
        self.keys = {}              # label -> (poly, texte)
        self.by_item = {}
        self.held = set()
        self.hover = None
        for r, row in enumerate(ROWS):
            for label, scan, ext, w in row:
                poly = self.create_polygon(0, 0, 0, 0, smooth=True, fill=KEY, outline="")
                text = self.create_text(0, 0, text=label.strip(), fill=TXT, font=self.fnt)
                self.keys[label] = (poly, text)
                self.by_item[poly] = self.by_item[text] = (label, scan, ext)
        self.bind("<Configure>", self._on_configure)
        self.bind("<Motion>", self._on_motion)
        self.bind("<Leave>", lambda e: self._set_hover(None))
        self.bind("<Button-1>", self._on_click)
        self._pending = None
        self._width = 0

    def _on_configure(self, e):
        self._width = e.width
        if self._pending is None:          # regroupe les événements : un seul tracé par image
            self._pending = self.after_idle(self.layout)

    def layout(self):
        self._pending = None
        W, g = max(self._width, 1), self.GAP * self.sc
        kh = self.KEY_H * self.sc
        rad = 8 * self.sc
        for r, row in enumerate(ROWS):
            total, cum = sum(k[3] for k in row), 0.0
            y0, y1 = r * (kh + g) + g / 2, r * (kh + g) + g / 2 + kh
            for label, scan, ext, w in row:
                x0, x1 = cum / total * W + g / 2, (cum + w) / total * W - g / 2
                cum += w
                poly, text = self.keys[label]
                self.coords(poly, *self._rr(x0, y0, x1, y1, min(rad, (x1 - x0) / 2)))
                self.coords(text, (x0 + x1) / 2, (y0 + y1) / 2)

    @staticmethod
    def _rr(x0, y0, x1, y1, r):
        return [x0 + r, y0, x0 + r, y0, x1 - r, y0, x1 - r, y0, x1, y0, x1, y0 + r, x1, y0 + r, x1, y1 - r,
                x1, y1 - r, x1, y1, x1 - r, y1, x1 - r, y1, x0 + r, y1, x0 + r, y1, x0, y1, x0, y1 - r,
                x0, y1 - r, x0, y0 + r, x0, y0 + r, x0, y0]

    def _paint(self, label):
        poly, text = self.keys[label]
        held = label in self.held
        self.itemconfigure(poly, fill=ORANGE if held else (KEY_HOV if label == self.hover else KEY))
        self.itemconfigure(text, fill="#111" if held else TXT)

    def set_held(self, labels):
        old, self.held = self.held, set(labels)
        for label in old ^ self.held:
            self._paint(label)

    def _key_at(self, e):
        items = self.find_overlapping(e.x, e.y, e.x, e.y)
        return self.by_item[items[-1]][0] if items and items[-1] in self.by_item else None

    def _set_hover(self, label):
        if label != self.hover:
            old, self.hover = self.hover, label
            for l in (old, label):
                if l is not None:
                    self._paint(l)

    def _on_motion(self, e):
        self._set_hover(self._key_at(e))

    def _on_click(self, e):
        label = self._key_at(e)
        if label is not None:
            self.on_key(label, *[x for x in self.by_item[self.keys[label][0]][1:]])


class SimpleScroll(tk.Frame):
    """Zone défilante légère (Canvas + Frame) : beaucoup plus rapide à redimensionner que CTkScrollableFrame.
    La barre de défilement n'apparaît que si le contenu dépasse."""

    def __init__(self, parent, scale):
        super().__init__(parent, bg=BG, bd=0, highlightthickness=0)
        self.bar_w = max(int(8 * scale), 6)
        self.canvas = tk.Canvas(self, bg=BG, highlightthickness=0, bd=0)
        self.bar = tk.Canvas(self, bg=BG, width=self.bar_w, highlightthickness=0, bd=0, cursor="hand2")
        self.inner = tk.Frame(self.canvas, bg=BG, bd=0, highlightthickness=0)
        self.win = self.canvas.create_window(0, 0, window=self.inner, anchor="nw")
        self.canvas.pack(side="left", fill="both", expand=True)
        self.bar_shown = False
        self.canvas.bind("<Configure>", self._on_canvas)
        self.inner.bind("<Configure>", self._on_inner)
        self.bar.bind("<Button-1>", self._drag)
        self.bar.bind("<B1-Motion>", self._drag)
        self.bind_all("<MouseWheel>", self._wheel, add="+")
        self._job = None

    def _on_canvas(self, e):
        self.canvas.itemconfigure(self.win, width=e.width)
        self._schedule()

    def _on_inner(self, e):
        self._schedule()

    def _schedule(self):
        if self._job is None:
            self._job = self.after_idle(self._update)

    def _update(self):
        self._job = None
        ch, vh = self.inner.winfo_reqheight(), self.canvas.winfo_height()
        self.canvas.configure(scrollregion=(0, 0, 1, max(ch, vh)))
        need = ch > vh + 1
        if need != self.bar_shown:
            self.bar_shown = need
            if need:
                self.bar.pack(side="right", fill="y", before=self.canvas)
            else:
                self.bar.pack_forget()
                self.canvas.yview_moveto(0)
        if need:
            self._draw_bar(ch, vh)

    def _draw_bar(self, ch, vh):
        self.bar.delete("all")
        bh = self.bar.winfo_height()
        top, bottom = self.canvas.yview()
        self.bar.create_rectangle(1, top * bh, self.bar_w - 1, max(bottom * bh, top * bh + 20),
                                  fill=BORDER, outline="")

    def _drag(self, e):
        top, bottom = self.canvas.yview()
        self.canvas.yview_moveto(max(0.0, e.y / max(self.bar.winfo_height(), 1) - (bottom - top) / 2))
        self._draw_bar(0, 0)

    def _wheel(self, e):
        if self.bar_shown and self.winfo_containing(e.x_root, e.y_root) is not None:
            self.canvas.yview_scroll(-1 if e.delta > 0 else 1, "units")
            self._draw_bar(0, 0)


def card(parent, title=None):
    f = ctk.CTkFrame(parent, fg_color=CARD, corner_radius=16)
    f.pack(fill="x", pady=(0, 10))
    if title:
        ctk.CTkLabel(f, text=title.upper(), font=font(11, "bold"), text_color=MUTED).pack(anchor="w", padx=16,
                                                                                        pady=(12, 0))
    return f


class App(ctk.CTk):
    def __init__(self):
        ctk.set_appearance_mode("dark")
        super().__init__(fg_color=BG)
        self.title("AutoKey – touche à heure précise")
        self.minsize(960, 520)
        sc = self._get_window_scaling()
        self.geometry(f"1100x{min(800, int(self.winfo_screenheight() / sc) - 70)}+40+10")
        try:
            ctypes.windll.shell32.SetCurrentProcessExplicitAppUserModelID("Clemzy.AutoKey")
        except Exception:
            pass
        self.after(300, self.set_icon)       # customtkinter pose sa propre icône : on passe après
        self.target_win = None   # cible : fenêtre + position de la zone de saisie
        self.actions = []        # liste d'actions planifiées (mode liste)
        self.mods = {}           # modificateurs maintenus : label -> (scan, ext)
        self.sel = None          # index de la ligne sélectionnée dans la liste
        self.rows_ui = []
        self.cancel = threading.Event()
        self.running = False
        self.build()
        self.build_list()
        self.load()
        self.refresh()
        self.bind_all("<Button-1>", self.click_anywhere, add="+")
        self.protocol("WM_DELETE_WINDOW", self.quit_app)
        self.after(100, self.fit_height)

    def set_icon(self):
        try:
            self.iconbitmap(asset("autokey.ico"))
        except tk.TclError:
            pass

    # ================= interface =================
    def build(self):
        # --- barre du bas (toujours visible) : boutons + état ---
        bottom = self.bottom_frame = ctk.CTkFrame(self, fg_color=CARD, corner_radius=0)
        bottom.pack(side="bottom", fill="x")
        bar = self.bar = ctk.CTkFrame(bottom, fg_color="transparent")
        bar.pack(fill="x", padx=14, pady=8)
        self.go = ctk.CTkButton(bar, text="▶  Armer", width=140, height=40, corner_radius=12, font=font(14, "bold"),
                                fg_color=GREEN, hover_color=GREEN_HOV, text_color="#06210f", command=self.arm)
        self.go.pack(side="left")
        self.stop = ctk.CTkButton(bar, text="■  Annuler", width=120, height=40, corner_radius=12,
                                  font=font(14, "bold"), fg_color=RED, hover_color=RED_HOV, state="disabled",
                                  command=self.cancel_run)
        self.stop.pack(side="left", padx=8)
        for text, cmd in (("Effacer", self.clear), ("Test (3 s)", self.test)):
            ctk.CTkButton(bar, text=text, height=40, corner_radius=12, font=font(13), fg_color=KEY,
                          hover_color=KEY_HOV, text_color=TXT, command=cmd).pack(side="left", padx=(0, 8))
        self.status = ctk.CTkLabel(bar, text="Arrêt d'urgence : Ctrl + Alt + Échap", font=font(12),
                                   text_color=MUTED, anchor="e")
        self.status.pack(side="right", padx=(8, 4))

        # --- en-tête ---
        head = self.head_frame = ctk.CTkFrame(self, fg_color="transparent")
        head.pack(side="top", fill="x", padx=20, pady=(10, 2))
        logo = ctk.CTkImage(Image.open(asset("autokey.png")), size=(40, 40))
        ctk.CTkLabel(head, image=logo, text="").pack(side="left")
        titles = ctk.CTkFrame(head, fg_color="transparent")
        titles.pack(side="left", padx=12)
        ctk.CTkLabel(titles, text="AutoKey", font=font(22, "bold"), text_color=TXT).pack(anchor="w")
        ctk.CTkLabel(titles, text="Une touche, un texte, à la milliseconde près.", font=font(11),
                     text_color=MUTED).pack(anchor="w")
        self.head_right = ctk.CTkFrame(head, fg_color="transparent")
        self.head_right.pack(side="right")
        ctk.CTkButton(self.head_right, text="💙  Soutenir " + AUTHOR.split(" aka ")[0], height=36, corner_radius=18,
                      font=font(13, "bold"), fg_color="#0070ba", hover_color="#1a8ad6", text_color="white",
                      command=lambda: webbrowser.open(DONATE_URL)).pack(side="left", padx=(0, 18))
        self.list_mode = tk.BooleanVar(value=False)
        ctk.CTkSwitch(self.head_right, text="📋  Mode liste d'actions", variable=self.list_mode,
                      command=self.toggle_list, font=font(13, "bold"), progress_color=ACCENT,
                      button_hover_color=ACCENT_HOV).pack(side="left")

        # --- contenu défilant ---
        self.scroller = SimpleScroll(self, self._get_window_scaling())
        self.scroller.pack(fill="both", expand=True, padx=10, pady=(2, 0))
        self.main = self.scroller.inner

        # clavier AZERTY : les touches se partagent toute la largeur, proportionnellement
        kb = card(self.main)
        self.kbd = KeyboardCanvas(kb, self._get_window_scaling(), self.toggle)
        self.kbd.pack(fill="x", padx=10, pady=10)

        # programmation : quand + où dans une seule carte
        prog = card(self.main)
        row = ctk.CTkFrame(prog, fg_color="transparent")
        row.pack(fill="x", padx=14, pady=(10, 4))
        now = datetime.now() + timedelta(minutes=1)
        self.h, self.m, self.s = (tk.StringVar(value=f"{v:02d}") for v in (now.hour, now.minute, 0))
        self.ms = tk.StringVar(value="000")
        ctk.CTkLabel(row, text="Heure", font=font(13), text_color=MUTED).pack(side="left", padx=(0, 8))
        for var, hi, w, dg, st in ((self.h, 23, 56, 2, 1), (self.m, 59, 56, 2, 1), (self.s, 59, 56, 2, 1),
                                   (self.ms, 999, 70, 3, 50)):
            if var is self.ms:
                ctk.CTkLabel(row, text=".", font=font(18, "bold"), text_color=MUTED).pack(side="left", padx=1)
            num_entry(row, var, 0, hi, width=w, digits=dg, step=st).pack(side="left")
            if var in (self.h, self.m):
                ctk.CTkLabel(row, text=":", font=font(18, "bold"), text_color=MUTED).pack(side="left", padx=1)
        ctk.CTkLabel(row, text="ms", font=font(12), text_color=MUTED).pack(side="left", padx=(4, 18))

        self.use_date = tk.BooleanVar(value=False)
        ctk.CTkSwitch(row, text="Date", variable=self.use_date, command=self.refresh, width=64, font=font(13),
                      progress_color=ACCENT, button_hover_color=ACCENT_HOV).pack(side="left")
        self.date = tk.StringVar(value=datetime.now().strftime("%d/%m/%Y"))
        self.date_en = ctk.CTkEntry(row, textvariable=self.date, width=108, height=38, corner_radius=10,
                                    justify="center", font=font(14, "bold", "Consolas"), fg_color=FIELD,
                                    border_color=BORDER, text_color=TXT)
        self.date_en.pack(side="left", padx=(6, 18))

        self.repeat = tk.BooleanVar(value=False)
        ctk.CTkSwitch(row, text="Répéter", variable=self.repeat, command=self.refresh, width=80, font=font(13),
                      progress_color=ACCENT, button_hover_color=ACCENT_HOV).pack(side="left")
        self.rep, self.gap = tk.StringVar(value="10"), tk.StringVar(value="100")
        self.rep_en = num_entry(row, self.rep, 2, 9999, width=64, digits=0, wrap=False, size=15)
        self.rep_en.pack(side="left", padx=(6, 6))
        ctk.CTkLabel(row, text="× toutes les", font=font(12), text_color=MUTED).pack(side="left")
        self.gap_en = num_entry(row, self.gap, 0, 60000, width=72, digits=0, wrap=False, step=50, size=15)
        self.gap_en.pack(side="left", padx=6)
        ctk.CTkLabel(row, text="ms", font=font(12), text_color=MUTED).pack(side="left")

        row = ctk.CTkFrame(prog, fg_color="transparent")
        row.pack(fill="x", padx=14, pady=(2, 10))
        self.minim = tk.BooleanVar(value=True)
        self.use_target = tk.BooleanVar(value=True)
        self.back = tk.BooleanVar(value=True)
        # (variable, libellé, couleur quand activée)
        self.opt_defs = [(self.minim, "Réduire la fenêtre au lancement", VIOLET),
                         (self.use_target, "Aller dans cette zone avant d'écrire", JAUNE),
                         (self.back, "Revenir ensuite où j'étais", ORANGE)]
        ctk.CTkButton(row, text="🎯  Choisir la zone de saisie", height=36, corner_radius=10, font=font(13, "bold"),
                      fg_color=ACCENT, hover_color=ACCENT_HOV, command=self.pick_target).pack(side="left")
        ctk.CTkButton(row, text="✕", width=36, height=36, corner_radius=10, font=font(13, "bold"), fg_color=KEY,
                      hover_color=KEY_HOV, command=self.clear_target).pack(side="left", padx=(6, 12))
        self.opt_btn = ctk.CTkButton(row, text="⚙  Options  ▾", height=36, corner_radius=10, font=font(13, "bold"),
                                     fg_color=KEY, hover_color=KEY_HOV, command=self.open_options)
        self.opt_btn.pack(side="left")
        self.opt_squares = []
        for _ in self.opt_defs:     # 3 petits carrés : gris = option désactivée, couleur = activée
            sq = ctk.CTkFrame(row, width=20, height=20, corner_radius=6, fg_color=GRIS)
            sq.pack(side="left", padx=(8, 0))
            self.opt_squares.append(sq)
        self.tgt_label = ctk.CTkLabel(row, text="", font=font(12), text_color=MUTED, anchor="w")
        self.tgt_label.pack(side="left", fill="x", expand=True, padx=(14, 0))

        # quoi
        what = card(self.main)
        self.entry = ctk.CTkEntry(what, height=42, corner_radius=12, fg_color="white", text_color="#111",
                                  border_color=ACCENT, border_width=2, font=font(16, "normal", "Consolas"),
                                  placeholder_text="Tape ton texte ici… les touches cliquées s'ajoutent entre [crochets]",
                                  placeholder_text_color="#8a8fa8")
        self.entry.pack(fill="x", padx=14, pady=(10, 2))
        self.entry.bind("<KeyRelease>", lambda e: self.refresh())
        self.info = ctk.CTkLabel(what, text="", font=font(12), text_color=MUTED, anchor="w")
        self.info.pack(fill="x", padx=16, pady=(0, 8))

    # ================= liste d'actions =================
    def build_list(self):
        self.list_frame = ctk.CTkFrame(self.main, fg_color=CARD, corner_radius=16)
        ctk.CTkLabel(self.list_frame, text="ACTIONS PLANIFIÉES", font=font(11, "bold"), text_color=MUTED
                     ).pack(anchor="w", padx=16, pady=(12, 0))
        ctk.CTkLabel(self.list_frame, text="Règle une action avec les cartes du dessus, puis ajoute-la. "
                     "Double-clic sur une ligne pour la recharger.", font=font(12), text_color=MUTED,
                     anchor="w").pack(fill="x", padx=16)
        head = ctk.CTkFrame(self.list_frame, fg_color="transparent")
        head.pack(fill="x", padx=16, pady=(8, 0))
        for text, w in (("Options", 100), ("Quand", 230), ("À envoyer", 0), ("Cible", 150), ("Répét.", 60)):
            ctk.CTkLabel(head, text=text, font=font(11, "bold"), text_color=MUTED, width=w, anchor="w"
                         ).pack(side="left", fill="x", expand=(w == 0))
        self.rows_box = ctk.CTkFrame(self.list_frame, fg_color="transparent")
        self.rows_box.pack(fill="x", padx=12, pady=4)
        btns = ctk.CTkFrame(self.list_frame, fg_color="transparent")
        btns.pack(fill="x", padx=12, pady=(4, 14))
        for text, cmd in (("➕  Ajouter l'action ci-dessus", self.add_action),
                          ("⟳  Remplacer la ligne sélectionnée", self.replace_action),
                          ("🗑  Supprimer", self.delete_action), ("Tout vider", self.clear_actions)):
            ctk.CTkButton(btns, text=text, height=36, corner_radius=10, font=font(13), fg_color=KEY,
                          hover_color=KEY_HOV, command=cmd).pack(side="left", padx=4)

    def toggle_list(self):
        if self.list_mode.get():
            self.list_frame.pack(fill="x", pady=(0, 10))
            self.go.configure(text="▶  Armer la liste")
        else:
            self.list_frame.pack_forget()
            self.go.configure(text="▶  Armer")
        self.after(50, self.fit_height)

    def fit_height(self):
        """Ajuste la hauteur de la fenêtre au contenu (dans la limite de l'écran)."""
        self.update_idletasks()
        sc = self._get_window_scaling()
        need = (self.head_frame.winfo_reqheight() + self.main.winfo_reqheight() + self.bottom_frame.winfo_reqheight()
                + 44) / sc
        h = max(520, min(int(need), int(self.winfo_screenheight() / sc) - 70))
        self.geometry(f"{int(self.winfo_width() / sc)}x{h}")

    def snapshot(self):
        """Capture les réglages actuels en une action ; None si invalides (message affiché)."""
        if self.parse() is None:
            return None
        if not (self.entry.get() or self.mods):
            messagebox.showwarning("AutoKey", "Rien à envoyer : tape du texte ou clique une touche.")
            return None
        try:
            act = {"h": int(self.h.get()), "m": int(self.m.get()), "s": int(self.s.get()), "ms": int(self.ms.get()),
                   "use_date": self.use_date.get(), "date": self.date.get().strip(), "text": self.entry.get(),
                   "mods": list(self.mods), "repeat": self.repeat.get(), "rep": int(self.rep.get()),
                   "gap": int(self.gap.get()), "target": self.target_win, "use_target": self.use_target.get(),
                   "back": self.back.get(), "minim": self.minim.get()}
            self.when(act)
        except ValueError as err:
            messagebox.showerror("AutoKey", f"Valeurs invalides (date : JJ/MM/AAAA). - {err}")
            return None
        return act

    def load_act(self, act):
        for var, key in ((self.h, "h"), (self.m, "m"), (self.s, "s")):
            var.set(f"{act[key]:02d}")
        self.ms.set(f"{act['ms']:03d}")
        self.use_date.set(act["use_date"])
        self.date.set(act["date"])
        self.entry.delete(0, tk.END)
        self.entry.insert(0, act["text"])
        self.mods = {l: MODS[l] for l in act["mods"] if l in MODS}
        self.repeat.set(act["repeat"])
        self.rep.set(str(act["rep"]))
        self.gap.set(str(act["gap"]))
        self.target_win = act["target"]
        self.use_target.set(act["use_target"])
        self.back.set(act["back"])
        self.minim.set(act.get("minim", True))
        self.refresh()

    def draw_list(self):
        for w in self.rows_box.winfo_children():
            w.destroy()
        self.rows_ui = []
        for i, a in enumerate(self.actions):
            r = ctk.CTkFrame(self.rows_box, fg_color=ROW, corner_radius=10, height=44)
            r.pack(fill="x", pady=2)
            r.pack_propagate(False)
            sq = ctk.CTkFrame(r, fg_color="transparent", width=100)
            sq.pack(side="left", padx=(8, 0))
            sq.pack_propagate(False)
            for on, color in zip((a.get("minim", True), a["use_target"], a["back"]), (VIOLET, JAUNE, ORANGE)):
                ctk.CTkFrame(sq, width=22, height=22, corner_radius=6, fg_color=color if on else GRIS
                             ).pack(side="left", padx=(0, 6), pady=11)
            when = f"{a['date'] if a['use_date'] else 'prochain'} {a['h']:02d}:{a['m']:02d}:{a['s']:02d}.{a['ms']:03d}"
            tgt = a["target"]["exe"] if a["target"] and a["use_target"] else "fenêtre active"
            labels = []
            for text, w, grow, fnt in ((when, 230, False, font(13, "bold", "Consolas")),
                                       (a["text"] or "(touches maintenues)", 0, True, font(13)),
                                       (tgt, 150, False, font(13)), (f"×{a['rep']}" if a["repeat"] else "-", 60, False,
                                                                     font(13))):
                lb = ctk.CTkLabel(r, text=text, width=w, anchor="w", font=fnt, text_color=TXT)
                lb.pack(side="left", fill="x", expand=grow)
                labels.append(lb)
            for wdg in (r, sq, *labels):
                wdg.bind("<Button-1>", lambda e, i=i: self.select(i))
                wdg.bind("<Double-Button-1>", lambda e, i=i: self.load_selected(i))
            self.rows_ui.append(r)
        self.sel = None if self.sel is None or self.sel >= len(self.actions) else self.sel
        self.select(self.sel)

    def select(self, i):
        self.sel = i
        for k, r in enumerate(self.rows_ui):
            r.configure(fg_color=ROW_SEL if k == i else ROW)

    def add_action(self):
        act = self.snapshot()
        if act:
            self.actions.append(act)
            self.draw_list()

    def replace_action(self):
        act = self.snapshot() if self.sel is not None else None
        if act:
            self.actions[self.sel] = act
            self.draw_list()

    def delete_action(self):
        if self.sel is not None:
            del self.actions[self.sel]
            self.draw_list()

    def clear_actions(self):
        self.actions.clear()
        self.draw_list()

    def load_selected(self, i):
        if not self.running:
            self.load_act(self.actions[i])

    # ================= menu Options (3 interrupteurs colorés) =================
    def open_options(self):
        pop = getattr(self, "opt_pop", None)
        if pop is not None and pop.winfo_exists():
            return self.close_options()
        pop = self.opt_pop = ctk.CTkFrame(self, fg_color=CARD, corner_radius=14, border_width=1, border_color=BORDER)
        self.opt_pop_btns = []
        for var, text, color in self.opt_defs:
            b = ctk.CTkButton(pop, text=text, anchor="w", height=42, corner_radius=10, font=font(13, "bold"),
                              command=lambda v=var: self.flip_option(v))
            b.pack(fill="x", padx=8, pady=4)
            self.opt_pop_btns.append(b)
        # place() de customtkinter attend des unités indépendantes de l'écran : on convertit les pixels
        sc = self._get_window_scaling()
        pop.update_idletasks()
        x = self.opt_btn.winfo_rootx() - self.winfo_rootx()
        top = self.opt_btn.winfo_rooty() - self.winfo_rooty()
        y = top + self.opt_btn.winfo_height() + 6 * sc
        if y + pop.winfo_reqheight() > self.winfo_height() - 120 * sc:   # pas la place dessous : ouvre vers le haut
            y = top - pop.winfo_reqheight() - 6 * sc
        pop.place(x=x / sc, y=y / sc)
        pop.lift()
        self.refresh_opts()

    def click_anywhere(self, event):
        """Ferme le menu Options quand on clique en dehors."""
        pop = getattr(self, "opt_pop", None)
        if pop is None or not pop.winfo_exists():
            return
        w = str(event.widget)
        if not (w.startswith(str(pop)) or w.startswith(str(self.opt_btn))):
            self.close_options()

    def close_options(self):
        pop = getattr(self, "opt_pop", None)
        if pop is not None and pop.winfo_exists():
            pop.destroy()

    def flip_option(self, var):
        var.set(not var.get())
        self.refresh_opts()

    def refresh_opts(self):
        for (var, _, color), sq in zip(self.opt_defs, self.opt_squares):
            sq.configure(fg_color=color if var.get() else GRIS)
        pop = getattr(self, "opt_pop", None)
        if pop is not None and pop.winfo_exists():
            for (var, text, color), b in zip(self.opt_defs, self.opt_pop_btns):
                on = var.get()
                b.configure(fg_color=color if on else KEY, hover_color=color if on else KEY_HOV,
                            text_color="#111" if on else TXT, text=("✔   " if on else "☐   ") + text)

    # ================= cible : fenêtre + zone de saisie =================
    def pick_target(self):
        if self.running:
            return
        self.withdraw()
        self.hint = tk.Toplevel(self)
        self.hint.overrideredirect(True)
        self.hint.attributes("-topmost", True)
        self.hint.geometry("+20+20")
        tk.Label(self.hint, text="🎯  Clique dans la zone de saisie à cibler  (Échap = annuler)",
                 bg="#d9822b", fg="white", font=("Segoe UI", 14, "bold"), padx=16, pady=10).pack()
        self.hint.update_idletasks()
        self.after(300, self.poll_pick)

    def poll_pick(self):
        if user32.GetAsyncKeyState(VK_ESCAPE) & 0x8000:
            return self.end_pick(None)
        if user32.GetAsyncKeyState(VK_LBUTTON) & 0x8000:
            x, y = cursor_pos()
            h = root_at(x, y)
            own = {int(self.hint.winfo_id()), user32.GetAncestor(int(self.hint.winfo_id()), GA_ROOT)}
            if h and h not in own:
                l, t, r, b = win_rect(h)
                info = {"hwnd": h, "exe": win_exe(h), "cls": win_class(h), "title": win_title(h),
                        "dx": x - l, "dy": y - t, "w": r - l, "h": b - t,
                        "fx": (x - l) / max(r - l, 1), "fy": (y - t) / max(b - t, 1)}
                return self.end_pick(info)
        self.after(20, self.poll_pick)

    def end_pick(self, info):
        self.hint.destroy()
        self.deiconify()
        if info:
            self.target_win = info
            self.use_target.set(True)
            self.save()
        self.refresh()

    def clear_target(self):
        self.target_win = None
        self.refresh()

    # ================= saisie =================
    def toggle(self, label, scan, ext):
        if self.running:
            return
        if label in MODS:
            self.mods.pop(label) if label in self.mods else self.mods.__setitem__(label, (scan, ext))
        else:
            self.entry.insert(self.entry.index(tk.INSERT), f"[{label.strip()}]")
        self.refresh()

    def clear(self):
        if not self.running:
            self.entry.delete(0, tk.END)
            self.mods.clear()
            self.refresh()

    def parse(self, text=None):
        """'bonjour[Entrée]' -> [('char','b'), ..., ('key',(scan,ext))]; None si touche inconnue."""
        out = []
        for i, part in enumerate(re.split(r"\[([^\]]+)\]", self.entry.get() if text is None else text)):
            if i % 2 == 0:
                out += [("char", c) for c in part]
            elif part.strip().lower() in LOOKUP:
                out.append(("key", LOOKUP[part.strip().lower()]))
            else:
                messagebox.showerror("AutoKey", f"Touche inconnue : [{part}]")
                return None
        return out

    def refresh(self):
        self.refresh_opts()
        self.kbd.set_held(self.mods)
        self.date_en.configure(state="normal" if self.use_date.get() else "disabled")
        for e in (self.rep_en, self.gap_en):
            e.configure(state="normal" if self.repeat.get() else "disabled")
        t = self.target_win
        self.tgt_label.configure(text=(f"Cible : {t['exe']} – « {t['title'][:70]} »  (point {t['dx']}, {t['dy']})"
                                       if t else "Aucune cible : la touche part dans la fenêtre active."))
        mods = " + ".join(m.strip() for m in self.mods)
        self.info.configure(text=("Touches maintenues : " + mods) if mods else
                            "Clique une touche du clavier pour l'insérer dans la ligne, ou tape du texte directement.")

    # ================= exécution =================
    def press_once(self, items, mods):
        for scan, ext in mods.values():
            send_scan(scan, ext, False)
        for kind, v in items:
            if self.cancel.is_set():
                break
            if kind == "char":
                send_char(v)
            else:
                send_scan(v[0], v[1], False)
                time.sleep(0.02)
                send_scan(v[0], v[1], True)
            time.sleep(0.02)
        for scan, ext in reversed(list(mods.values())):
            send_scan(scan, ext, True)

    def when(self, act):
        """Prochain instant correspondant à l'heure (et à la date si demandée) d'une action."""
        t = datetime.now().replace(hour=act["h"], minute=act["m"], second=act["s"], microsecond=act["ms"] * 1000)
        if act["use_date"]:
            d = datetime.strptime(act["date"], "%d/%m/%Y")
            t = t.replace(year=d.year, month=d.month, day=d.day)
            if t <= datetime.now():
                raise ValueError("Cette date/heure est déjà passée.")
            return t
        return t if t > datetime.now() else t + timedelta(days=1)

    def make_job(self, act, ts=None):
        return {"ts": ts or self.when(act).timestamp(), "items": self.parse(act["text"]),
                "mods": {l: MODS[l] for l in act["mods"] if l in MODS},
                "n": act["rep"] if act["repeat"] else 1, "gap": act["gap"] / 1000 if act["repeat"] else 0,
                "target": act["target"] if act["use_target"] else None, "back": act["back"],
                "minim": act.get("minim", True)}

    def arm(self):
        if self.list_mode.get():
            if not self.actions:
                return messagebox.showwarning("AutoKey", "La liste est vide : ajoute au moins une action.")
            try:
                jobs = sorted((self.make_job(a) for a in self.actions), key=lambda j: j["ts"])
            except ValueError as err:
                return messagebox.showerror("AutoKey", f"Une action de la liste est invalide : {err}")
        else:
            act = self.snapshot()
            if not act:
                return
            jobs = [self.make_job(act)]
        self.save()
        self.start(jobs)

    def test(self):
        act = self.snapshot()
        if act:
            self.start([self.make_job(act, time.time() + 3)], "Test")

    def start(self, jobs, label=None):
        self.running = True
        self.cancel.clear()
        self.go.configure(state="disabled")
        self.stop.configure(state="normal")
        keep_awake(True)
        self.watch()
        threading.Thread(target=self.worker, args=(jobs, label), daemon=True).start()

    def watch(self):
        """Surveille le raccourci d'arrêt d'urgence tant qu'une action est en cours."""
        if not self.running:
            return
        if panic_pressed():
            self.cancel.set()
        self.after(40, self.watch)

    def set_status(self, text, color=MUTED):
        self.status.configure(text=text, text_color=color)

    def run_job(self, job):
        """Exécute une action ; renvoie un message d'erreur, ou None si tout s'est bien passé."""
        err = None
        prev, cur0 = user32.GetForegroundWindow(), cursor_pos()
        if job["target"]:
            try:
                click_target(job["target"])
            except RuntimeError as e:
                err = str(e)
        for i in range(0 if err else job["n"]):
            if self.cancel.is_set():
                break
            self.press_once(job["items"], job["mods"])
            if job["gap"] and i < job["n"] - 1:
                time.sleep(job["gap"])
        if job["target"] and job["back"] and not err:
            user32.SetCursorPos(*cur0)
            if prev and user32.IsWindow(prev):
                focus_window(prev)
        return err

    def worker(self, jobs, label):
        errors = []
        done = 0
        for k, job in enumerate(jobs):
            if job["minim"]:
                self.after(0, self.iconify)
            name = label or f"Action {k + 1}/{len(jobs)} à {datetime.fromtimestamp(job['ts']):%H:%M:%S}"
            # attente grossière, puis boucle serrée pour la précision à la milliseconde
            while not self.cancel.is_set():
                left = job["ts"] - time.time()
                if left <= 0:
                    break
                self.after(0, self.set_status, f"⏱  {name} – dans {left:6.1f} s", "#9ab4ff")
                time.sleep(min(0.25, left - 0.05) if left > 0.1 else 0)
            if self.cancel.is_set():
                break
            err = self.run_job(job)
            if err:
                errors.append(f"action {k + 1} : {err}")
            elif not self.cancel.is_set():
                done += 1
        if self.cancel.is_set():
            msg, color = "Annulé (arrêt d'urgence ou bouton).", "#ffb454"
        elif errors:
            msg, color = f"⚠  {done}/{len(jobs)} envoyée(s) – " + " ; ".join(errors), "#ff7b7b"
        else:
            msg = f"✔  Terminé à {datetime.now():%H:%M:%S.%f}"[:-3] + (f" ({done} actions)" if len(jobs) > 1 else "")
            color = "#5be08b"
        self.after(0, self.finish, msg, color)

    def cancel_run(self):
        self.cancel.set()

    def finish(self, msg, color=MUTED):
        self.running = False
        keep_awake(False)
        self.set_status(msg, color)
        self.go.configure(state="normal")
        self.stop.configure(state="disabled")
        self.deiconify()

    # ================= réglages =================
    def save(self):
        data = {"text": self.entry.get(), "mods": list(self.mods), "repeat": self.repeat.get(),
                "rep": self.rep.get(), "gap": self.gap.get(), "minim": self.minim.get(),
                "use_date": self.use_date.get(), "date": self.date.get(),
                "use_target": self.use_target.get(), "back": self.back.get(),
                "target": strip_hwnd(self.target_win), "list_mode": self.list_mode.get(),
                "actions": [dict(a, target=strip_hwnd(a["target"])) for a in self.actions]}
        try:
            os.makedirs(os.path.dirname(SETTINGS), exist_ok=True)
            with open(SETTINGS, "w", encoding="utf-8") as f:
                json.dump(data, f, ensure_ascii=False, indent=1)
        except OSError:
            pass

    def load(self):
        try:
            with open(SETTINGS, encoding="utf-8") as f:
                d = json.load(f)
        except (OSError, ValueError):
            return
        self.entry.insert(0, d.get("text", ""))
        for label in d.get("mods", []):
            if label in MODS:
                self.mods[label] = MODS[label]
        self.repeat.set(d.get("repeat", False))
        self.rep.set(d.get("rep", "10"))
        self.gap.set(d.get("gap", "100"))
        self.minim.set(d.get("minim", True))
        self.use_date.set(d.get("use_date", False))
        self.date.set(d.get("date", self.date.get()))
        self.use_target.set(d.get("use_target", True))
        self.back.set(d.get("back", True))
        self.target_win = d.get("target") or None
        self.actions = [a for a in d.get("actions", []) if isinstance(a, dict)]
        self.draw_list()
        if d.get("list_mode"):
            self.list_mode.set(True)
            self.toggle_list()

    def quit_app(self):
        self.save()
        self.destroy()


if __name__ == "__main__":
    App().mainloop()
