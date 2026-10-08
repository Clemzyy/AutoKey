# AutoKey

**Tape une touche, un texte ou une combinaison à l'heure exacte que tu choisis — dans la fenêtre de ton choix.**

🌐 [English](README.md) · **Français**

![AutoKey](docs/screenshot-fr.png)

AutoKey est un petit utilitaire Windows écrit en Rust : un seul fichier `.exe` d'environ 4,5 Mo, sans installation, qui démarre instantanément et dont l'interface reste fluide quand on redimensionne la fenêtre.

## Télécharger

➡️ **[Télécharger AutoKey.exe — dernière version](../../releases/latest)** (Windows 10/11 64 bits, sans installation).

Le fichier n'est pas signé : Windows SmartScreen peut avertir au premier lancement → *Informations complémentaires* → *Exécuter quand même*.

## Fonctions

- **Clavier à l'écran en 13 dispositions** — AZERTY (FR, BE), QWERTY (US, UK, ES, IT, BR), QWERTZ (DE, CH), russe ЙЦУКЕН, arabe, Dvorak et Colemak. La disposition de ta langue de saisie Windows est détectée automatiquement ; le chinois et le japonais utilisent le QWERTY (US).
- **Interface en 6 langues** — English, Français, Español, Русский, العربية, 中文. La langue suit celle de Windows et se change depuis l'en-tête.
- **Texte et touches dans une seule ligne** : `bonjour[Entrée]` tape « bonjour » puis appuie sur Entrée. Le texte est tapé en Unicode (accents, symboles, toute écriture). Une touche entre crochets (`[a]`, `[F5]`, `[Entrée]`) est une vraie pression de touche, traduite selon la disposition réellement active dans la fenêtre cible : elle fonctionne avec des modificateurs maintenus (Ctrl, Alt, Maj, Alt Gr).
- **Heure à la milliseconde**, date optionnelle, **répétition** optionnelle (nombre et intervalle).
- **Zone cible** : clique une fois dans le champ voulu ; AutoKey retrouve la fenêtre (même après un redémarrage de l'application visée), la restaure si elle est réduite, la ramène au premier plan, clique dans la zone, puis écrit. Il refuse d'écrire si la fenêtre est introuvable ou masquée, plutôt que de taper au hasard.
- **Trois options par action**, repérées par des carrés colorés : 🟣 réduire AutoKey au lancement · 🟡 aller dans la zone avant d'écrire · 🟠 revenir ensuite où tu étais.
- **Mode liste d'actions** : plusieurs actions, chacune avec son heure, son texte, sa cible et ses options, exécutées dans l'ordre des heures.
- **Arrêt d'urgence** : `Ctrl + Alt + Échap`, pris en compte à tout moment, même fenêtre réduite et pendant une longue pause entre deux répétitions. **Anti-veille** pendant qu'une action est armée.
- **Précision** : avec une cible, la fenêtre est préparée 1,5 s avant l'heure pour que la première touche parte à l'heure exacte (mesuré : +1 à +2 ms).
- **Sécurité** : une action en retard de plus de 30 s (PC en veille…) est ignorée plutôt que tapée dans la mauvaise fenêtre ; une seule instance ; réglages écrits de façon atomique ; message clair si Windows refuse les touches (cible lancée en administrateur).
- Champs numériques : clic pour taper, glisser, ou `Ctrl + molette`. Réglages mémorisés dans `%APPDATA%\AutoKey\reglages.json`.

> L'arabe est entièrement traduit, correctement lié et ordonné de droite à gauche, mais la mise en page de la fenêtre n'est pas inversée. Les traductions ont été écrites avec soin mais pas relues par des locuteurs natifs : les corrections sont les bienvenues (voir *Ajouter ou corriger une langue*).

## Démarrage rapide

1. Clique sur les touches du clavier à l'écran (ou tape ton texte) dans la ligne blanche.
2. Règle l'heure.
3. *(Optionnel)* **Choisir la zone de saisie**, puis clique dans le champ visé.
4. **Armer**. Le bouton **Test (3 s)** permet d'essayer tout de suite.

Les applications lancées **en administrateur** ignorent les touches envoyées par un programme normal : lance alors AutoKey en administrateur.

## Compiler

Prérequis : [Rust](https://rustup.rs) (toolchain MSVC) et les *Build Tools for Visual Studio* (composant « Développement Desktop en C++ »).

```bash
cargo build --release
```

L'exécutable est produit dans `target\release\autokey.exe` (ou dans le dossier indiqué par `.cargo/config.toml` s'il existe).

### Ajouter ou corriger une langue

Tous les textes sont dans un seul tableau, `src/i18n.rs` : chaque entrée contient ses six traductions côte à côte (vérifié à la compilation). Les dispositions de clavier sont dans `src/keys.rs`. Les contributions sont bienvenues.

### Remarque technique : `vendor/eframe`

`eframe` 0.36 crée sa fenêtre cachée avant d'initialiser OpenGL. Sur certains pilotes Intel récents (testé : Iris Xe, Windows 11 build 26300), l'initialisation OpenGL se bloque alors indéfiniment. `vendor/eframe` est une copie de `eframe 0.36.2` où **une seule ligne** change (`with_visible(true)` dans `src/native/glow_integration.rs`), branchée via `[patch.crates-io]` dans `Cargo.toml`. `eframe` est distribué sous licence MIT OU Apache-2.0 par l'équipe egui.

## Tests automatiques

`tests/ui_test.py` pilote la vraie fenêtre (souris et clavier réels) et vérifie 61 points : clavier, dispositions, langues, champs, options, mode liste, annulation, fermeture, envoi dans une fenêtre cible.

```bash
python tests/ui_test.py target/release/autokey.exe
```

| Variable d'environnement | Effet |
|---|---|
| `AUTOKEY_SETTINGS=chemin.json` | utilise un autre fichier de réglages |
| `AUTOKEY_LANG=fr` / `AUTOKEY_LAYOUT=azerty-fr` | force la langue / la disposition du clavier |
| `AUTOKEY_AUTOARM=N` ou `HH:MM:SS` | arme l'action dans N secondes (ou à l'heure donnée) dès le démarrage |
| `AUTOKEY_AUTOPICK=1` | lance le repérage de zone dès le démarrage |
| `AUTOKEY_SHOT=image.png` | enregistre une capture de la fenêtre puis quitte |
| `AUTOKEY_BENCH=1` | mesure le temps d'image pendant des redimensionnements (résultat dans `%TEMP%\autokey_bench.txt`) |
| `AUTOKEY_STATE=etat.json` | écrit l'état interne et la position de chaque composant (utilisé par `tests/ui_test.py`) |
| `AUTOKEY_SHOTS=dossier` | capture d'écran à la demande (fichier `req.txt` contenant le nom) |

## Soutenir le projet

Si AutoKey te rend service, tu peux soutenir son développement : [**💙 Faire un don via PayPal**](https://www.paypal.com/donate/?hosted_button_id=NKCR6KK739WGS)

## Auteur

**Clemzy** aka **InforMagicien** — licence [MIT](LICENSE).
