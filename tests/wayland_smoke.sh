#!/bin/bash
# Test de bout en bout sous Wayland : AutoKey tape dans gedit par le portail « RemoteDesktop ».
#
# Usage : bash tests/wayland_smoke.sh chemin/vers/autokey      (à lancer depuis un terminal de la session Wayland)
# Prérequis : une session Wayland (XDG_SESSION_TYPE=wayland), gedit, python3.
# Une fenêtre du système demande d'autoriser AutoKey : clique sur « Partager » / « Autoriser » dans les 40 secondes.
BIN=$(readlink -f "${1:-target/release/autokey}")
[ -x "$BIN" ] || { echo "exécutable introuvable : $BIN"; exit 2; }
[ "$XDG_SESSION_TYPE" == "wayland" ] || { echo "ce test doit tourner dans une session Wayland (XDG_SESSION_TYPE=$XDG_SESSION_TYPE)"; exit 2; }
for tool in gedit python3; do command -v "$tool" >/dev/null || { echo "outil manquant : $tool"; exit 2; }; done

T=$(mktemp -d /tmp/akwl.XXXXXX)
trap 'pkill -f "^$BIN( |\$)" 2>/dev/null; pkill -x gedit 2>/dev/null; rm -rf "$T"' EXIT

# deux actions : le texte (AutoKey se réduit alors, gedit reprend le focus), puis Ctrl + S pour enregistrer
python3 - "$T/s.json" <<'EOF'
import json, sys, time
now = time.time()
def act(offset, text, mods, minim):
    t = time.localtime(now + offset)
    return {"h": t.tm_hour, "m": t.tm_min, "s": t.tm_sec, "ms": 0, "use_date": False, "date": "", "text": text, "mods": mods,
            "repeat": False, "rep": 1, "gap": 100, "target": None, "use_target": False, "back": False, "minim": minim}
a1 = act(45, "Bonjour Wayland é€ ç @#[Enter]ligne2 жзы 你好[Enter]fin", [], True)
a2 = act(51, "s", ["ctrl"], False)
json.dump(dict(a1, list_mode=True, actions=[a1, a2]), open(sys.argv[1], "w"))
EOF
gedit "$T/out.txt" >/dev/null 2>&1 &
sleep 6
echo "AutoKey démarre : clique sur « Partager » dans la fenêtre du système (40 s)…"
AUTOKEY_SETTINGS="$T/s.json" AUTOKEY_LANG=fr AUTOKEY_AUTOARM=5 "$BIN" >"$T/ak.log" 2>&1 &
sleep 62
GOT=$(cat "$T/out.txt" 2>/dev/null)
WANT=$'Bonjour Wayland é€ ç @#\nligne2 жзы 你好\nfin'
if [ "$GOT" == "$WANT" ]; then
  echo "  OK     texte reçu à l'identique (majuscules, accents, Alt Gr, cyrillique, chinois, retours à la ligne)"
  exit 0
fi
echo "  ECHEC  texte reçu : $(printf '%q' "$GOT")"
exit 1
