# Lot C7d : bouton « Laisser en garnison »

Spec : ordre `Order::GarrisonUnits` (C7a, `core/crates/sim-campaign/src/orders.rs`),
demande C5/C7a « pas encore de bouton dans l'UI ». Branche :
`worktree-agent-af13b589dd9cd2655` (main `0a7bc32` fusionné).

## État

- [ ] `settlement_detail` (pont) : ajout `garrison_cap` et `garrison_free`.
- [ ] `ArmyStrip` : bouton « Garnison » (sélection multiple existante), tooltip française
  si assiégée ou pleine.
- [ ] `MapUI` / `HudController` : calcul de la disponibilité, émission de l'ordre
  `garrison_units`, toast d'erreur si refusé.
- [ ] Extension de `game/tests/c5_settlements_ui_test.gd`.
- [ ] `cargo fmt`, `clippy`, `cargo test`, `build.sh`, smoke + `settlements_render_test` +
  `c5_settlements_ui_test`.

## Décisions

- Bouton placé dans `ArmyStrip` (bandeau d'ost), pas dans `SettlementPanel` : l'armée est
  déjà sélectionnée avec ses régiments et la sélection multiple (Maj/Ctrl-clic) existe déjà
  pour `split_army`. Le panneau de colonie n'a pas la notion d'armée sélectionnée.
- `HudController.show_army` interroge `settlement_detail(army.location)` pour savoir si la
  colonie est contrôlée par le joueur, assiégée ou pleine ; toute la logique reste côté
  cœur (le pont ne fait qu'exposer `garrison_cap`/`garrison_free`, le GDScript ne fait que
  lire ces champs).

## Prochaine étape

Implémentation en cours.
