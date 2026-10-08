# AutoKey

**Tape une touche, un texte ou une combinaison à l'heure exacte que tu choisis — dans la fenêtre de ton choix.**

![AutoKey](docs/screenshot.png)

AutoKey est un petit utilitaire Windows écrit en Rust : un seul fichier `.exe` d'environ 4,4 Mo, sans installation,
qui démarre instantanément et dont l'interface reste fluide quand on redimensionne la fenêtre.

> La première version, écrite en Python, reste disponible dans l'historique du dépôt : tag [`v1.0-python`](../../tree/v1.0-python).

## Télécharger

➡️ **[AutoKey.exe — dernière version](../../releases/latest)** (Windows 10/11, ≈ 4,4 Mo, sans installation).
Windows SmartScreen peut avertir au premier lancement, car le fichier n'est pas signé : *Informations complémentaires* → *Exécuter quand même*.

## Fonctions

- Clavier **AZERTY** complet à l'écran : clique sur une touche pour l'ajouter. Ctrl, Alt, Maj et Alt Gr se maintiennent pendant l'envoi.
- **Texte + touches** dans une seule ligne : `bonjour[Entrée]` tape « bonjour » puis appuie sur Entrée.
- **Heure à la milliseconde**, date optionnelle, **répétition** optionnelle.
- **Zone cible** : clique une fois dans la zone de saisie voulue ; AutoKey retrouve la fenêtre (même réduite), la ramène au premier plan,
  clique dans la zone et écrit. Il refuse d'écrire si la fenêtre est introuvable ou masquée.
- **Options** par action : réduire la fenêtre au lancement · aller dans la zone avant d'écrire · revenir ensuite où tu étais.
- **Mode liste d'actions** : plusieurs actions, chacune avec son heure, son texte, sa cible et ses options.
- **Arrêt d'urgence** : `Ctrl + Alt + Échap` (pris en compte à tout moment, même pendant une longue pause entre deux répétitions). **Anti-veille** pendant qu'une action est armée.
- **Précision** : quand une cible est choisie, la fenêtre est préparée 1,5 s avant l'heure (restaurée, mise au premier plan, clic dans la zone) pour que la première touche parte à l'heure exacte (mesuré : +1 à +2 ms).
- **Sécurité** : une action en retard de plus de 30 s (PC en veille…) est ignorée plutôt que tapée au hasard ; une seule instance à la fois ; réglages écrits de façon atomique ; message clair si Windows refuse les touches (fenêtre lancée en administrateur).
- Champs numériques : clic pour taper, glisser, ou `Ctrl + molette`.
- Réglages mémorisés dans `%APPDATA%\AutoKey\reglages.json` (même format que la version Python 1.0).

## Compiler

Prérequis : [Rust](https://rustup.rs) (toolchain MSVC) et les *Build Tools for Visual Studio* (composant « Développement Desktop en C++ »).

```bash
cargo build --release
```

L'exécutable est produit dans `target\release\autokey.exe` (ou dans le dossier indiqué par `.cargo/config.toml` s'il existe).

## Remarque technique : `vendor/eframe`

`eframe` 0.36 crée sa fenêtre cachée avant d'initialiser OpenGL. Sur certains pilotes Intel récents (testé : Iris Xe, Windows 11 build 26300),
l'initialisation OpenGL se bloque alors indéfiniment. Le dossier `vendor/eframe` est une copie de `eframe 0.36.2` où **une seule ligne** change
(`with_visible(true)` dans `src/native/glow_integration.rs`). Elle est branchée via `[patch.crates-io]` dans `Cargo.toml`.
`eframe` est distribué sous licence MIT OU Apache-2.0 par l'équipe egui.

## Tests automatiques (variables d'environnement)

| Variable | Effet |
|---|---|
| `AUTOKEY_SETTINGS=chemin.json` | utilise un autre fichier de réglages |
| `AUTOKEY_AUTOARM=N` ou `HH:MM:SS` | arme l'action dans N secondes (ou à l'heure donnée) dès le démarrage |
| `AUTOKEY_AUTOPICK=1` | lance le repérage de zone dès le démarrage |
| `AUTOKEY_SHOT=image.png` | enregistre une capture de la fenêtre puis quitte |
| `AUTOKEY_BENCH=1` | mesure le temps d'image pendant des redimensionnements (résultat dans `%TEMP%\autokey_bench.txt`) |
| `AUTOKEY_STATE=etat.json` | écrit l'état interne et la position de chaque composant (utilisé par `tests/ui_test.py`) |
| `AUTOKEY_SHOTS=dossier` | capture d'écran à la demande (fichier `req.txt` contenant le nom) |

`tests/ui_test.py` pilote la vraie fenêtre (souris et clavier réels) et vérifie 49 points : clavier, champs, options, mode liste, annulation, fermeture, envoi dans une fenêtre cible.

```bash
python tests/ui_test.py target/release/autokey.exe
```

## Soutenir le projet

[**Faire un don via PayPal**](https://www.paypal.com/donate/?hosted_button_id=NKCR6KK739WGS)

## Auteur

**Clemzy** aka **InforMagicien** — licence [MIT](LICENSE).
