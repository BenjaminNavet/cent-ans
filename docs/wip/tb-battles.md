# TB — historique des batailles (note de reprise)

Lot du chantier TB (`docs/design/2026-10-02-campagne-tob.md`, « restes » de TB4). Branche
`feat/tb-battles`, worktree `/Users/jean_hubert/dev/gp-tb8`. Build privé :
`export CARGO_TARGET_DIR=/Users/jean_hubert/dev/gp-tb8/core/target`.

But : les marques de champ de bataille (`game/scripts/map/war_scars.gd`, TB4, ADR 0157) survivent
au rechargement. Le cœur garde un historique borné des batailles terrestres ; le pont le rend ; le
rendu le lit.

## État
- [x] Squelette : `data_model::BattleHistoryRules` (`data/rules/battle_history.json`, schéma
  `battle_history_rules`), `sim_campaign::battle_history` (structure, enregistrement, purge),
  champ `CampaignState::battle_history` (`serde(default)`), appels dans `movement.rs`
  (bataille rangée), `siege.rs` (assaut, sortie), `turn.rs` (purge), méthode du pont
  `get_battle_history` (vide), tests Rust désactivés.
- [ ] Tests Rust (enregistrement, purge, aller-retour, ancienne sauvegarde).
- [ ] Pont : `get_battle_history` rempli.
- [ ] `war_scars.gd` : lecture de l'historique, repli sur l'ancienne déduction.
- [ ] `game/tests/tb4_scars_test.gd` : bataille, sauvegarde, rechargement, âge, échéance.
- [ ] ADR 0157 (révision), `docs/wip/tb4.md`, pytest du schéma.

## Prochaine étape
Écrire les tests Rust dans `core/crates/sim-campaign/tests/tb_battle_history.rs`.
