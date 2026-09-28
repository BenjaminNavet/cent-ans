# RS-F — Bataille : incendie et sélecteur de formations (fichier de reprise)

Lot F du chantier RS (`docs/wip/restes.md`). Branche `feat/rs-f-battle`. ADR réservé : 0099.
Cible cargo privée : `CARGO_TARGET_DIR=<worktree>/core/target-rs-f` (à supprimer à la fin).

## Points

1. Ordre « Incendier » dans l'UI (S2, `s1-s2-physique-incendies.md`).
2. Règles de feu lues depuis `data/rules/siege_fire.json` au chargement (plus seulement `include_str!`).
3. Sélecteur de formations CB6 repliable (`cb.md`).
4. Tests Rust + Godot.

## État

- Cœur : `FireRules::{from_json, load, install, current}` (`sim-battle/src/fire.rs`) ; `FireSystem::new`
  prend `FireRules::current()` (installées, sinon intégrées = défaut des tests).
  `BattleSim::burn_choice(side, units)` (`sim/fire.rs`) : cible la plus proche à portée de torche, même
  test que la commande `burn` (factorisé dans `torch_distance`, aucune règle changée, aucun tirage).
  Erreurs nouvelles : `NothingLeftToBurn`, `NothingInReach`.
- Pont : `install_battle_rules(data_dir)` (`campaign_sim.rs`) appelé au chargement disque des données
  et par `historical_battles::battle_data` ; `BattleSim.get_burn_order(side, units)`.
- UI : bouton « Incendier » en bout de `leader_orders_bar.gd` (sièges seulement, touche physique I,
  infobulle riche, grisé avec la raison du cœur) ; ligne d'aide dans `battle_hotkeys.gd`.
- `battle_formation_picker.gd` : en-tête cliquable « ▾/▸ Formations de groupe », `collapsed` gardé par
  le nœud (vit toute la bataille).

- ADR 0099 écrit (aucune règle changée ; mécanisme de chargement + ordre).
- Tests : `sim-battle/tests/rs_f_burn_choice.rs` (5), `rs_f_fire_rules_data.rs` (1, seul dans son
  binaire car `install` est global au processus) ; `game/tests/rs_f_battle_test.gd` ; `cargo test`
  complet vert, clippy propre ; smoke, cb2, cb4, cb6, s2_fire_fx, ep13 verts.

## État : TERMINÉ (en attente de fusion par l'orchestrateur)

Limites : le rejeu (EP13) ne stocke pas les règles de feu ; seul le feu est lu depuis `data/` (les
autres `*Rules::bundled()` restent intégrées) ; l'ordre vise la cible la plus proche (pas de clic
sur une maison précise) ; glyphe ♨ en attendant une icône DA `order_burn`. Remarque hors lot :
`leader_orders_bar.gd::give` cherche `_available_selection` sur la scène, qui ne l'a pas (elle est
dans `battle_input.gd`) : les ordres « sélection » visent donc toujours tous les régiments.
