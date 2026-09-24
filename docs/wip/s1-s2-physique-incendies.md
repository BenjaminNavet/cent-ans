# WIP — S1 effondrement des murailles + S2 incendies de siège

Demande du joueur (24/09) : physique de destruction des murailles et incendies.

## État
- **S1 (effondrement physique, rendu seulement)** : **fusionné dans main** (`41bb147`). Jolt activé, ADR 0007,
  `game/scripts/battle/wall_collapse_fx.gd`, réglages `data/fx/siege_fx.json`. Détails : `docs/wip/s1-effondrement-murailles.md`.
- **S2 (incendies, règle du cœur + rendu)** : **fusionné dans main** (`67a5aae`, 24/09).
  - Conflit `battle_siege.gd` (S1 `_fx` + S2 `fire_fx`) et conflit `sim.rs` avec B5 (`generate_site` + `let mut siege`) résolus.
  - Vérifié : fmt, clippy, cargo test, smoke Godot, `wall_collapse_fx_test`, `s2_fire_fx_test`, pytest des schémas.
  - Détails, règles et mesures : `docs/design/s2-incendies.md`, ADR 0008, `docs/wip/s2-incendies.md`.

## Prochaine étape
- Capture d'un siège en feu (pas faite : mode silencieux `~/.cent-ans-quiet` interdit les fenêtres Godot).

## Limites connues à traiter plus tard
- S1 : porte de Guyenne masquée par ses tours (géométrie antérieure) ; pas de son d'effondrement.
- S2 : règles embarquées par `include_str!` (recompiler pour changer un réglage) ; l'IA n'évite pas les rues en feu
  et n'utilise `burn` que pour les faubourgs ; pas de bouton « incendier » dans l'UI ; l'église ne s'effondre pas
  visuellement ; pas de lutte contre le feu.
