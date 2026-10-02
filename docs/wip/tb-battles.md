# TB — historique des batailles (note de reprise)

Lot du chantier TB (« restes » de TB4, `docs/wip/tb4.md`). Branche `feat/tb-battles`, worktree
`/Users/jean_hubert/dev/gp-tb8`. Build privé :
`export CARGO_TARGET_DIR=/Users/jean_hubert/dev/gp-tb8/core/target`.

But : les marques de champ de bataille (`game/scripts/map/war_scars.gd`, TB4) survivent au
rechargement. Décision : ADR 0157, section « Révision ».

## État
- [x] `data_model::BattleHistoryRules` (`core/crates/data-model/src/entities/battle_history.rs`),
  lu dans `data/rules/battle_history.json` (`max_age_turns` 8, `max_records` 256), schéma
  `data/schemas/battle_history_rules.schema.json`.
- [x] `sim_campaign::battle_history` (`core/crates/sim-campaign/src/battle_history.rs`) :
  `BattleRecord` (tour, province, position, nature, camps avec effectif et pertes, vainqueur),
  `BattleHistory` (ajout, purge par âge puis par nombre), `record`, `on_new_turn`.
- [x] Champ `CampaignState::battle_history` (`serde(default)`, non écrit s'il est vide,
  `STATE_VERSION` inchangé).
- [x] Enregistrement : `movement::apply_battle_result` (bataille rangée, auto ou 3D),
  `siege::apply_assault_result` (assaut), `siege::sortie` (sortie) ; purge dans
  `turn.rs` après `advance_date`. Quatre petits ajouts dans des fichiers partagés.
- [x] Pont : `CampaignSim.get_battle_history()`
  (`core/crates/godot-bridge/src/campaign_sim_battle_history.rs`).
- [x] `war_scars.gd` : `_read_history` (une marque par province, à sa dernière bataille ; pas de
  marque sans mort) ; ancienne déduction gardée en repli quand le pont n'a pas la méthode.
- [x] Tests Rust `core/crates/sim-campaign/tests/tb_battle_history.rs` (9) : bataille auto,
  bataille 3D, assaut, sortie, purge (unité et tours réels), aller-retour de sauvegarde,
  sauvegarde sans le champ, règles lues dans les données.
- [x] `game/tests/tb4_scars_test.gd` (point 3 bis) : vraie simulation, bataille, tour,
  sauvegarde, rechargement dans une simulation et un rendu neufs, âge, échéance, sauvegarde sans
  la clé.
- [x] pytest `tools/tests/test_war_scars_ui_schema.py` : schéma des bornes,
  `battlefield.turns ≤ max_age_turns`.
- [x] ADR 0157 (révision), `docs/wip/tb4.md`.

## Vérifications (02/10)
- `cargo test -p sim-campaign --test tb_battle_history` : 9 OK.
- `tb4_scars_test.gd` : OK (bataille au tour 0 en Île-de-France, marque partie au tour 4, à sa
  propre échéance) ; `smoke.gd` : OK ; pytest `test_war_scars_ui_schema` : 6 OK.
- `cargo test` complet : voir « Points ouverts » tant que la ligne n'est pas remplacée.

## Points ouverts
- `cargo test` complet : en cours au moment de ce commit.
- `cargo clippy -- -D warnings` échoue sur la base avec rustc 1.99 :
  `crates/godot-bridge/src/historical_battles.rs:57` (`godot_warn!` en position d'expression,
  lint `semicolon_in_expressions_from_non_local_macros`). Fichier hors lot, non touché ; le reste
  est propre avec `-A semicolon_in_expressions_from_non_local_macros`.
- Une marque par province (comme TB4) : deux batailles en deux points d'une même province ne
  donnent qu'un tertre (la dernière). Une marque par bataille est possible avec l'historique ; à
  juger à l'œil (encombrement).
- Assaut et sortie : la marque est au camp des assiégeants (position de l'armée), donc au pied
  de la place ; non jugé à l'œil.
- Sauvegarde d'avant l'historique : pas de marque pour les batailles de son dernier tour.
- `game/.godot` copié de main : `MapScale` manquait au cache des classes (erreurs de compilation
  au premier lancement) ; `godot --headless --path game --import` le répare.

## Prochaine étape
Attendre la fin de `cargo test`, corriger s'il y a lieu, puis commit final. Ensuite : fusion et
jugement à l'œil par la session principale.
