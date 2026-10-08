# AutoKey

**Type a key, a text or a shortcut at the exact time you choose — in the window you choose.**

🌐 **English** · [Français](README.fr.md)

![AutoKey](docs/screenshot.png)

AutoKey is a small Windows utility written in Rust: a single `.exe` of about 4.5 MB, no installation, instant startup, and an interface that stays smooth when you resize the window.

## Download

➡️ **[Download AutoKey.exe — latest release](../../releases/latest)** (Windows 10/11, 64-bit, no installation).

The file is not code-signed, so Windows SmartScreen may warn you on first launch: click *More info* → *Run anyway*.

## Features

- **On-screen keyboard in 13 layouts** — AZERTY (FR, BE), QWERTY (US, UK, ES, IT, BR), QWERTZ (DE, CH), Russian ЙЦУКЕН, Arabic, Dvorak and Colemak. The layout of your Windows input language is detected automatically; Chinese and Japanese users get QWERTY (US).
- **Interface in 6 languages** — English, Français, Español, Русский, العربية, 中文. The language follows Windows and can be changed from the header.
- **Text and keys in one line**: `hello[Enter]` types "hello" then presses Enter. Text is typed as Unicode (accents, symbols, any script). A key in brackets (`[a]`, `[F5]`, `[Enter]`) is a real key press, translated through the layout actually active in the target window, so it works with held modifiers (Ctrl, Alt, Shift, AltGr).
- **Time down to the millisecond**, with an optional date and an optional **repeat** (count and interval).
- **Target area**: click once in the input field you want; AutoKey finds that window again (even after a restart of the target application), restores it if minimized, brings it to the front, clicks the area, then types. It refuses to type if the window cannot be found or is covered, rather than typing at random.
- **Three options per action**, shown as colored squares: 🟣 minimize AutoKey on start · 🟡 go to the target area before typing · 🟠 then return to where you were.
- **Action list mode**: schedule several actions, each with its own time, text, target and options. They run in time order.
- **Emergency stop**: `Ctrl + Alt + Esc`, honored at any moment, even when the window is minimized and during a long pause between repeats. **Sleep prevention** while an action is armed.
- **Precision**: with a target, the window is prepared 1.5 s before the time so the first key is sent at the exact time (measured: +1 to +2 ms).
- **Safety**: an action more than 30 s late (PC asleep…) is skipped instead of being typed into the wrong window; single instance; settings written atomically; a clear message if Windows refuses the keys (target running as administrator).
- Numeric fields: click to type, drag, or `Ctrl + mouse wheel`. Settings are remembered in `%APPDATA%\AutoKey\reglages.json`.

> Arabic is fully translated and correctly shaped and ordered right-to-left, but the window layout itself is not mirrored. The translations were written with care but not reviewed by native speakers: corrections are very welcome (see *Adding or fixing a language* below).

## Quick start

1. Click keys on the on-screen keyboard (or type your text) in the white line.
2. Set the time.
3. *(Optional)* **Pick the input area**, then click in the field you want to target.
4. **Arm**. The **Test (3 s)** button lets you try right away.

Applications running **as administrator** ignore keys sent by a normal program: run AutoKey as administrator in that case.

## Build from source

Requirements: [Rust](https://rustup.rs) (MSVC toolchain) and the *Build Tools for Visual Studio* ("Desktop development with C++").

```bash
cargo build --release
```

The executable is produced in `target\release\autokey.exe` (or in the folder set by `.cargo/config.toml`, if present).

### Adding or fixing a language

All texts are in one table, `src/i18n.rs`: each entry lists its six translations side by side (checked at compile time). Keyboard layouts are in `src/keys.rs`. Pull requests welcome.

### Technical note: `vendor/eframe`

`eframe` 0.36 creates its window hidden before initializing OpenGL. On some recent Intel drivers (tested: Iris Xe, Windows 11 build 26300) OpenGL initialization then hangs forever. `vendor/eframe` is a copy of `eframe 0.36.2` where **one line** differs (`with_visible(true)` in `src/native/glow_integration.rs`), wired in through `[patch.crates-io]` in `Cargo.toml`. `eframe` is distributed under MIT OR Apache-2.0 by the egui team.

## Automated tests

`tests/ui_test.py` drives the real window (real mouse and keyboard) and checks 61 points: keyboard, layouts, languages, fields, options, list mode, cancellation, closing, typing into a target window.

```bash
python tests/ui_test.py target/release/autokey.exe
```

| Environment variable | Effect |
|---|---|
| `AUTOKEY_SETTINGS=path.json` | use another settings file |
| `AUTOKEY_LANG=en` / `AUTOKEY_LAYOUT=qwerty-us` | force the language / keyboard layout |
| `AUTOKEY_AUTOARM=N` or `HH:MM:SS` | arm the action in N seconds (or at the given time) at startup |
| `AUTOKEY_AUTOPICK=1` | start the target picker at startup |
| `AUTOKEY_SHOT=image.png` | save a screenshot of the window, then quit |
| `AUTOKEY_BENCH=1` | measure frame time while resizing (result in `%TEMP%\autokey_bench.txt`) |
| `AUTOKEY_STATE=state.json` | write the internal state and every component's position (used by `tests/ui_test.py`) |
| `AUTOKEY_SHOTS=folder` | on-demand screenshots (a `req.txt` file containing the name) |

## Support the project

If AutoKey is useful to you, you can support its development: [**💙 Donate via PayPal**](https://www.paypal.com/donate/?hosted_button_id=NKCR6KK739WGS)

## Author

**Clemzy** aka **InforMagicien** — [MIT license](LICENSE).
