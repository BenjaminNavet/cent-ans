# QW-H — icône d'application et image Open Graph

État : fait (hors points ci-dessous).

- Script : `uv run --project tools python tools/brand_assets.py` (Pillow, IM Fell English, couleurs de `front_end_style.gd`). Régénère tout.
- Icône : `game/icon.png` (1024), `game/icon.icns` (16-1024), `game/icon.ico` (16-256 ; filet et points d'or retirés sous 48 px, bordure épaissie), copie `tools/launcher-windows/icon.ico`.
- Branché : `config/icon="res://icon.png"` dans `project.godot` ; `application/icon` (macOS : .icns ; Windows : .ico) et `console_wrapper_icon` dans `export_presets.cfg`. Godot importe `icon.png` au prochain lancement (pas de `--import` fait ici).
- Open Graph : `docs/img/readme/og.png` (1280x640, recadrage de `menu.jpg` à droite du menu, lettrine, titre, accroche). **Téléversement manuel** par le propriétaire : GitHub, Settings > General > Social preview.

## Restes
- Windows : `application/modify_resources` reste à `false` ; l'icône n'est intégrée à l'`.exe` exporté que si `rcedit` (et `wine` hors Windows) est installé, puis passer à `true`.
- Lanceur `Lancer Cent Ans.exe` : icône embarquée (10-09). `tools/launcher-windows/build.rs` compile `icon.rc` (`1 ICON "icon.ico"`) avec `embed-resource` 3 (build-dependency) uniquement si la cible est windows ; `cargo xwin` depuis macOS trouve `llvm-rc` (Homebrew llvm), section `.rsrc` de 57 Ko vérifiée dans l'exe reconstruit. Non vérifié : affichage de l'icône dans l'Explorateur (seul un vrai Windows le montre) et build natif sur runner CI.
