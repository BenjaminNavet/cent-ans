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

## Prochaine étape

Compiler, tests Rust (`burn_choice`, chargement), test Godot `rs_f_battle_test.gd`, smoke.
