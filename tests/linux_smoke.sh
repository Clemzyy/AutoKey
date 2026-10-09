#!/bin/bash
# Tests de bout en bout sous Linux (session X11) : frappe réelle, ciblage d'une fenêtre, précision de l'heure, arrêt d'urgence.
#
# Usage : bash tests/linux_smoke.sh chemin/vers/autokey
# Prérequis : une session X11 ouverte (variable DISPLAY), et les outils xdotool, wmctrl, xev (x11-apps), gedit, python3.
# Les tests utilisent leurs propres réglages (AUTOKEY_SETTINGS) : tes réglages ne sont pas touchés.
BIN=$(readlink -f "${1:-target/release/autokey}")
[ -x "$BIN" ] || { echo "exécutable introuvable : $BIN"; exit 2; }
[ -n "$DISPLAY" ] || { echo "DISPLAY n'est pas défini : lance ce script dans une session graphique X11"; exit 2; }
for tool in xdotool wmctrl xev gedit python3; do
  command -v "$tool" >/dev/null || { echo "outil manquant : $tool"; exit 2; }
done

T=$(mktemp -d /tmp/akxt.XXXXXX)
FAILS=0
cleanup() { pkill -f "$BIN" 2>/dev/null; pkill -x xev 2>/dev/null; pkill -x gedit 2>/dev/null; }
trap cleanup EXIT
pass() { echo "  OK     $1"; }
fail() { echo "  ECHEC  $1  -> $2"; FAILS=$((FAILS + 1)); }

# réglages d'une action : $1 = texte, $2 = répétition (true/false), $3 = nombre, $4 = pause en ms
settings() {
  cat > "$T/s.json" <<EOF
{"h":0,"m":0,"s":0,"ms":0,"use_date":false,"date":"","text":"$1","mods":[],"repeat":${2:-false},"rep":${3:-1},"gap":${4:-100},"target":null,"use_target":false,"back":false,"minim":false}
EOF
}
run_ak() {  # $1 = variables d'environnement supplémentaires
  env AUTOKEY_SETTINGS="$T/s.json" AUTOKEY_LANG=fr AUTOKEY_LAYOUT=azerty-fr "$@" "$BIN" >"$T/ak.log" 2>&1 &
}

echo "== 1. frappe réelle (texte, accents, symboles, caractères hors disposition, touche nommée)"
settings 'Bonjour Z, ça va ? 123 @#&é€ [Enter]Ligne 2 жзы 你好[Enter]fin'
rm -f "$T/out.txt"
gedit --new-window "$T/out.txt" >/dev/null 2>&1 &
sleep 5
run_ak AUTOKEY_AUTOARM=8
sleep 3; wmctrl -a out.txt; sleep 12
xdotool key ctrl+s; sleep 2
GOT=$(cat "$T/out.txt" 2>/dev/null)
WANT=$'Bonjour Z, ça va ? 123 @#&é€ \nLigne 2 жзы 你好\nfin'
[ "$GOT" == "$WANT" ] && pass "texte reçu à l'identique" || fail "texte reçu" "$(printf '%q' "$GOT")"
cleanup; sleep 1

echo "== 2. cible : repérage dans la fenêtre A, B passe devant, la frappe arrive dans A"
settings 'CIBLE[Enter]ok'
rm -f "$T/a.txt" "$T/b.txt"
gedit --new-window "$T/a.txt" >/dev/null 2>&1 & sleep 4
gedit --new-window "$T/b.txt" >/dev/null 2>&1 & sleep 4
wmctrl -r a.txt -e 0,40,40,640,420; wmctrl -r b.txt -e 0,720,40,640,420; sleep 1
GEO=$(wmctrl -lG | grep " a.txt" | head -1)
AX=$(echo "$GEO" | awk '{print $3 + $5/2}' | cut -d. -f1); AY=$(echo "$GEO" | awk '{print $4 + $6/2}' | cut -d. -f1)
run_ak AUTOKEY_AUTOPICK=1
sleep 4; xdotool mousemove "$AX" "$AY" click 1; sleep 3
cleanup; sleep 1
python3 - "$T/s.json" <<'EOF' && pass "cible enregistrée (programme et classe de la fenêtre)" || fail "cible enregistrée" "aucune cible dans les réglages"
import json, sys
t = json.load(open(sys.argv[1])).get("target")
sys.exit(0 if t and t.get("exe") == "gedit" else 1)
EOF
gedit --new-window "$T/a.txt" >/dev/null 2>&1 & sleep 4
gedit --new-window "$T/b.txt" >/dev/null 2>&1 & sleep 4
wmctrl -r a.txt -e 0,40,40,640,420; wmctrl -r b.txt -e 0,720,40,640,420; sleep 1
wmctrl -a b.txt; sleep 1
run_ak AUTOKEY_AUTOARM=6
sleep 14
xdotool key ctrl+s; sleep 2
wmctrl -a b.txt; xdotool key ctrl+s; sleep 1
[ "$(cat "$T/a.txt" 2>/dev/null)" == $'CIBLE\nok' ] && pass "texte arrivé dans la fenêtre visée" || fail "texte dans A" "$(printf '%q' "$(cat "$T/a.txt" 2>/dev/null)")"
[ ! -s "$T/b.txt" ] && pass "l'autre fenêtre est restée vide" || fail "fenêtre B" "elle a reçu du texte"
cleanup; sleep 1

echo "== 3. précision de l'heure (la cible de frappe est la fenêtre xev, qui date chaque touche)"
settings 'x'
TARGET_STR=$(date -d "+12 seconds" +%H:%M:%S); TARGET_EPOCH=$(date -d "$TARGET_STR" +%s)
xev -event keyboard >"$T/xev.log" 2>&1 &
sleep 2
run_ak AUTOKEY_AUTOARM="$TARGET_STR"
sleep 3; wmctrl -a "Event Tester"; sleep 1
T0=$(date +%s.%N); xdotool key F12      # repère entre l'heure réelle et l'heure du serveur X
sleep 14
python3 - "$T/xev.log" "$T0" "$TARGET_EPOCH" <<'EOF' && pass "écart à l'heure demandée sous 10 ms" || fail "précision" "écart trop grand ou touche non reçue"
import re, sys
txt = open(sys.argv[1], errors="replace").read()
t0, target = float(sys.argv[2]), float(sys.argv[3])
ev = re.findall(r"KeyPress event.*?time (\d+)", txt, re.S)
if len(ev) < 2:
    sys.exit(1)
delta = (t0 + (int(ev[1]) - int(ev[0])) / 1000.0 - target) * 1000
print("         écart mesuré : %+.1f ms" % delta)
sys.exit(0 if abs(delta) < 10 else 1)
EOF
cleanup; sleep 1

echo "== 4. arrêt d'urgence (50 répétitions toutes les 200 ms, Ctrl + Alt + Échap maintenu 0,3 s après ~2 s)"
settings 'y' true 50 200
TARGET_STR=$(date -d "+10 seconds" +%H:%M:%S)
xev -event keyboard >"$T/xev.log" 2>&1 &
sleep 2
run_ak AUTOKEY_AUTOARM="$TARGET_STR"
sleep 3; wmctrl -a "Event Tester"; sleep 1
sleep $(( $(date -d "$TARGET_STR" +%s) - $(date +%s) + 2 ))
xdotool keydown ctrl keydown alt keydown Escape; sleep 0.3; xdotool keyup Escape keyup alt keyup ctrl
sleep 4
N=$(python3 - "$T/xev.log" <<'EOF'
import sys
blocks = open(sys.argv[1], errors="replace").read().split("\n\n")
print(sum(1 for b in blocks if b.lstrip().startswith("KeyPress") and "keysym 0x79" in b))
EOF
)
[ "$N" -ge 3 ] && [ "$N" -lt 30 ] && pass "arrêt obtenu après $N frappes sur 50" || fail "arrêt d'urgence" "$N frappes reçues sur 50"
cleanup

rm -rf "$T"
echo
[ "$FAILS" -eq 0 ] && echo "Tous les tests passent." || echo "$FAILS test(s) en échec."
exit "$FAILS"
