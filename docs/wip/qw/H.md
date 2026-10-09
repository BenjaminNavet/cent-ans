# QW-H — icône d'application et image Open Graph

État : fait (hors points ci-dessous).

- Script : `uv run --project tools python tools/brand_assets.py` (Pillow, IM Fell English, couleurs de `front_end_style.gd`). Régénère tout.
- Icône : `game/icon.png` (1024), `game/icon.icns` (16-1024), `game/icon.ico` (16-256 ; filet et points d'or retirés sous 48 px, bordure épaissie), copie `tools/launcher-windows/icon.ico`.
- Branché : `config/icon="res://icon.png"` dans `project.godot` ; `application/icon` (macOS : .icns ; Windows : .ico) et `console_wrapper_icon` dans `export_presets.cfg`. Godot importe `icon.png` au prochain lancement (pas de `--import` fait ici).
- Open Graph : `docs/img/readme/og.png` (1280x640, recadrage de `menu.jpg` à droite du menu, lettrine, titre, accroche). **Téléversement manuel** par le propriétaire : GitHub, Settings > General > Social preview.

## Restes
- Windows : `application/modify_resources` reste à `false` ; l'icône n'est intégrée à l'`.exe` exporté que si `rcedit` (et `wine` hors Windows) est installé, puis passer à `true`.
- Lanceur `Lancer Cent Ans.exe` : pas d'icône embarquée. `winresource`/`embed-resource` exigent `rc.exe`/`llvm-rc` ; la cross-compilation `cargo xwin` depuis macOS ne le garantit pas, et l'exe committé n'est pas reconstruit ici. Suite : ajouter `build.rs` + `winresource` (`set_icon("icon.ico")`) en CI Windows, ou `llvm-rc` installé, puis relancer `tools/launcher-windows/build.sh`.
