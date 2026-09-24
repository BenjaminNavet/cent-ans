# Lot C7d : bouton « Laisser en garnison »

Spec : ordre `Order::GarrisonUnits` (C7a, `core/crates/sim-campaign/src/orders.rs`),
demande C5/C7a « pas encore de bouton dans l'UI ». Branche :
`worktree-agent-af13b589dd9cd2655` (main `0a7bc32` fusionné).

## État : terminé (en attente de fusion par l'orchestrateur)

- [x] `settlement_detail` (pont) : ajout `garrison_cap` et `garrison_free`.
- [x] `ArmyStrip` : bouton « Garnison » (sélection multiple existante), tooltip française
  si assiégée ou pleine.
- [x] `MapUI` / `HudController` : calcul de la disponibilité, émission de l'ordre
  `garrison_units`, toast d'erreur si refusé.
- [x] Extension de `game/tests/c5_settlements_ui_test.gd` (étape 5).
- [x] `cargo fmt`, `clippy -D warnings`, `cargo test` : OK.
- [x] `build.sh`, `--import`, `smoke.gd`, `settlements_render_test.gd`,
  `c5_settlements_ui_test.gd` (avec l'étape 5, garnison) : tous OK.

## Décisions

- Bouton placé dans `ArmyStrip` (bandeau d'ost), pas dans `SettlementPanel` : l'armée est
  déjà sélectionnée avec ses régiments et la sélection multiple (Maj/Ctrl-clic) existe déjà
  pour `split_army`. Le panneau de colonie n'a pas la notion d'armée sélectionnée.
- `HudController.show_army` interroge `settlement_detail(army.location)` pour savoir si la
  colonie est contrôlée par le joueur, assiégée ou pleine ; toute la logique reste côté
  cœur (le pont ne fait qu'exposer `garrison_cap`/`garrison_free`, le GDScript ne fait que
  lire ces champs).

## Points ouverts

- Pas de test Rust dédié pour les deux nouveaux champs de `settlement_detail` : `CampaignSim`
  est un objet `#[godot_api]`, pas testable hors moteur ; le calcul (`garrison_cap.get`) est
  déjà exercé par `orders.rs` (plafond de l'ordre), le test Godot `c5_settlements_ui_test.gd`
  vérifie les champs et l'ordre bout en bout.
- Bouton ajouté sous « Séparer » dans l'en-tête du bandeau (VBoxContainer) : la hauteur du
  bandeau grandit légèrement (pas de capture visuelle demandée pour ce lot).

## Prochaine étape

Terminé : fusion par l'orchestrateur.
