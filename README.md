# AutoKey

**Tape une touche, un texte ou une combinaison à l'heure exacte que tu choisis — dans la fenêtre de ton choix.**

![AutoKey](docs/screenshot.png)

AutoKey est un petit utilitaire Windows : tu programmes ce qui doit être tapé et à quelle milliseconde, et il s'en charge, même si tu regardes une vidéo ou que la bonne fenêtre n'est pas au premier plan.

## Fonctions

- ⌨️ **Clavier AZERTY complet** à l'écran : clique sur une touche pour l'ajouter. Ctrl, Alt, Maj et Alt Gr se maintiennent pendant l'envoi.
- ✍️ **Texte + touches** dans une seule ligne : `bonjour[Entrée]` tape « bonjour » puis appuie sur Entrée.
- ⏱️ **Heure à la milliseconde**, avec une date optionnelle.
- 🔁 **Répétition** optionnelle (nombre de fois et intervalle).
- 🎯 **Zone cible** : clique une fois dans la zone de saisie voulue ; AutoKey retrouve la fenêtre (même réduite), la ramène au premier plan, clique dans la zone et écrit. Il refuse d'écrire si la fenêtre est introuvable ou masquée.
- ⚙️ **Options** (par action) : réduire la fenêtre au lancement · aller dans la zone avant d'écrire · revenir ensuite où tu étais.
- 📋 **Mode liste d'actions** : programme plusieurs actions, chacune avec son heure, son texte, sa cible et ses options.
- 🛑 **Arrêt d'urgence** : `Ctrl + Alt + Échap`, même si la fenêtre est réduite.
- 😴 **Anti-veille** : Windows ne se met pas en veille tant qu'une action est armée.
- 💾 Tes réglages sont mémorisés.

## Installation

Prérequis : Windows 10/11 et [Python 3.10+](https://www.python.org/downloads/).

```bash
git clone https://github.com/Clemzyy/AutoKey.git
cd AutoKey
pip install -r requirements.txt
python autokey.py
```

Sous Windows, tu peux aussi double-cliquer sur `lancer_autokey.bat`.

## Utilisation rapide

1. Clique sur les touches du clavier (ou tape ton texte) dans la ligne blanche.
2. Règle l'heure.
3. *(Optionnel)* **🎯 Choisir la zone de saisie**, puis clique dans la zone voulue.
4. **▶ Armer**. Le bouton **Test (3 s)** permet d'essayer tout de suite.

> Les jeux et applications lancés en administrateur peuvent ignorer les touches envoyées : lance alors AutoKey en administrateur.

## Soutenir le projet

Si AutoKey te rend service, tu peux soutenir son développement :
[**💙 Faire un don via PayPal**](https://www.paypal.com/donate/?hosted_button_id=NKCR6KK739WGS)

## Auteur

**Clemzy** aka **InforMagicien**

## Licence

[MIT](LICENSE)
