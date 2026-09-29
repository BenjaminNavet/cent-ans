# NT2 — Bataille personnalisée

Branche `feat/nt2-custom-battle`. Spec : `docs/superpowers/specs/2026-09-29-nt-nuit-tww3-design.md` (ligne NT2).

## Plan
- Cœur : `sim-battle/src/custom.rs` (configuration, roster, budget, validation, `BattleSetup`),
  météo forcée via `ReplayStart.weather` ; règles `data/rules/custom_battle.json` (+ schéma).
- Pont : `godot-bridge/src/custom_battle.rs` (`BattleSim.custom_factions/custom_roster/validate_custom/setup_custom`).
- UI : `custom_battle_screen.gd` + scène ; entrée du menu ; `BattleScene.custom_config` ;
  dernière composition dans `Settings` (`custom_battle/last`).
- Tests : `nt2_custom_battle_test.gd`, `nt2_shot.gd`, tests Rust `sim-battle/tests/nt2_custom.rs`.

## État
- [x] Squelette (écran vide, entrée du menu, test SKIPPED)
- [x] Cœur `sim-battle/src/custom.rs` + tests `nt2_custom.rs` ; météo forcée (`ReplayStart.weather`,
  `BattleSim::new_scaled_weather`) ; règles `data/rules/custom_battle.json` + schéma + pytest
- [x] Pont `godot-bridge/src/custom_battle.rs`
- [x] Écran complet, `BattleScene.custom_config` / `begin_custom` / `--custom-battle`, réglage
  `custom_battle/last`, infobulles `custom_battle_*` dans `data/ui/tooltips.json`
- [x] `nt2_custom_battle_test.gd` vert

- [x] `nt2_shot.gd` (capture `docs/audit/captures/nt/nt2-custom-battle.png`, non relue)
- [x] smoke vert ; po_ui_test C2 vert (marge haute du menu 24→16, écart sous le titre 16→8)
- [x] clippy -D warnings ; pytest du schéma

## Prochaine étape
Fusion dans `integration/nt`. Jugement visuel de l'écran par la session principale.

## Points ouverts
- Siège : place générique (niveau de fortification 0-3) ; le type de place NT1 (château, bourg,
  cité) n'est pas choisissable tant que NT1 n'est pas fusionné (`setup.province = "custom"`).
- Roster : restrictions de faction et de culture seulement (époque et technologies ignorées).
- Capitaine générique (commandement 5) sur le régiment le plus cher, monté d'abord.
- Graine aléatoire à chaque lancement (clé `seed` de la configuration pour la fixer).
