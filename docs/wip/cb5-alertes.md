# CB5 — Alertes de bataille typées (état)

Branche : `feat/cb5-alerts` (depuis `main` @ be631979, qui contient `cad1ca05` — CB0 + CB-M1
seulement ; CB-M2/CB-M3/CB-M4/CB1 ne sont **pas** encore dans `main`, malgré `docs/archive/chantiers.md`).
Cible cargo privée : `core/target-cb5`.

## État
- [x] Squelette : `core/crates/sim-battle/src/alerts.rs` (`AlertKind`, `BattleAlert`, `AlertRules`
      chargée depuis `data/rules/battle_alerts.json`), module déclaré dans `lib.rs`.
- [x] `data/rules/battle_alerts.json` + `data/schemas/battle_alerts_rules.schema.json` +
      `tools/tests/test_battle_alerts_schema.py`.
- [x] Champ `alerts`/`alerts_read` sur `BattleSim`, méthode `alert()`, `take_new_alerts()`.
- [x] Émission aux points du plan : déroute, général tué/capturé, flanc (front montant, via
      `flanked_alerted: Vec<bool>` car `Unit.flanked` est remis à 0 à chaque tick), renforts,
      munitions, mur/porte rompus (point unique : `record_siege_transitions`).
- [x] Pont `godot-bridge` : `get_alerts()` + constantes `cb5_alert_*` (`RuleValues`).
- [x] `core/crates/sim-battle/tests/cb5_alerts.rs` (8 tests, tous verts : un par type + digest
      + rejeu inchangés).
- [x] `game/scripts/battle/battle_alerts_column.gd` (colonne, fusion, borne 5, glyphes dessinés
      en code) + branchement `battle_hud.gd` (`_build_alerts`) / `battle_scene.gd`
      (`get_alerts` dans la boucle HUD, `_on_alert_pinged`, audio branché après `battle_audio`).
- [x] `battle_minimap.gd` : `ping(world)` + repère pulsé (anneau, `_process` seulement pendant
      le pulsé).
- [x] Cris `BattleAudio.play_event` : seulement rout/general_down (les autres types n'ont pas de
      cri dans la spec) ; `_detect_events` de `battle_audio.gd` joue déjà ces deux mêmes sons sur
      transition d'état, donc un doublon éventuel est amorti par le cooldown/max_instances de
      l'événement (pas de garde explicite anti-doublon — limite connue, notée ici).
- [x] `game/tests/cb5_alerts_test.gd` : OK, exit 0, aucune SCRIPT ERROR (fusion, zone différente,
      hors fenêtre, borne 5 + priorité, expiration, clic → signal `pinged`, intégration
      `battle.tscn` : colonne dans `hud.root`, `get_alerts()` répond, clic → caméra + minicarte).
- [x] `game/tests/cb5_alerts_shot.gd --probe` : OK, exit 0, pas de SCRIPT ERROR ; colonne
      `[8,8,244×207]`, journal `[1240,70,366×101]`, bandeau du bas `[8,766,1584×128]`, aucun
      chevauchement, 5 lignes affichées (borne respectée). Panneau de comparaison CB-M4 non
      fusionné dans cette branche : pas vérifié, à refaire après sa fusion (cf. note dans le
      script et le rapport final).

## Décisions
- Mur/porte rompus : un seul point d'émission, `record_siege_transitions` (sim/siege_assault.rs),
  qui détecte déjà le front montant intact→rompu de chaque pièce pour `SiegeFxKind::GateBroken` /
  `WallBreached`. Évite de dupliquer l'alerte entre bélier, engin de siège et feu (trois causes,
  un seul événement).
- Général : `GeneralDown` émis dans `kill_general` (couvre mort au combat et « pas de quartier »)
  et dans la branche « fait prisonnier » (capture). Un seul type d'alerte pour les deux, comme la
  spec (pas de modèle de blessure).
- Rejeu EP13 : `alerts`/`alerts_read` ne sont pas dans `state_digest` (comme `events`) — flux de
  sortie pur, aucun impact sur le déterminisme. À vérifier par un test.

## État final
Lot CB5 complet sur cette branche : cœur, pont, données, Godot, tous les tests demandés qui
existent dans cette branche sont verts (voir rapport final de l'agent). `cargo fmt`,
`cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (136 groupes,
0 échec), `uv run --project tools pytest` (791 passed, 2 skipped pré-existants), `smoke.gd`,
`cb0_input_equivalence_test`, `cb_m1_outline_test`, `cb5_alerts_test`, `cb5_alerts_shot --probe`
tous OK (exit 0, aucune `SCRIPT ERROR`).

`cb1_drag_formation_test`, `cb_m2_path_hover_test`, `cb_m3_queue_test`,
`cb_m4_range_compare_test` : **absents de cette branche** (CB-M2/CB-M3/CB-M4/CB1 pas encore
fusionnés dans `main`, malgré `docs/archive/chantiers.md`) — impossible à exécuter ici, pas une régression
de CB5. Le panneau de comparaison au survol (CB-M4) n'a donc pas pu être vérifié contre le
chevauchement de la colonne d'alertes ; à refaire une fois CB-M4 fusionné (le probe script laisse
une note explicite à cet effet).

Touches ajoutées : aucune (pas de remappage, CB2 s'en charge). Fichiers `battle_hud.gd` /
`battle_scene.gd` : ajouts minimes et localisés (une constante, un champ, un appel de
construction, un branchement de signal, une ligne dans la boucle HUD) pour limiter les conflits
avec CB2/CB3/CB6 qui travaillent en parallèle sur ces mêmes fichiers.

Je n'ai pas mis à jour `docs/archive/chantiers.md` (fichier d'orchestration partagé, modifié par les autres
lots en parallèle) : à faire par la session d'orchestration lors de la fusion.
