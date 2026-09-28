# WIP — S1 effondrement des murailles + S2 incendies de siège

Demande du joueur (24/09) : physique de destruction des murailles et incendies.

## État
- **S1 (effondrement physique, rendu seulement)** : **fusionné dans main** (`41bb147`). Jolt activé, ADR 0007,
  `game/scripts/battle/wall_collapse_fx.gd`, réglages `data/fx/siege_fx.json`. Détails : `docs/wip/s1-effondrement-murailles.md`.
- **S2 (incendies, règle du cœur + rendu)** : **fusionné dans main** (`67a5aae`, 24/09).
  - Conflit `battle_siege.gd` (S1 `_fx` + S2 `fire_fx`) et conflit `sim.rs` avec B5 (`generate_site` + `let mut siege`) résolus.
  - Vérifié : fmt, clippy, cargo test, smoke Godot, `wall_collapse_fx_test`, `s2_fire_fx_test`, pytest des schémas.
  - Détails, règles et mesures : `docs/design/s2-incendies.md`, ADR 0008, `docs/wip/s2-incendies.md`.

## Captures (24/09)
- `game/tests/s2_fire_shot.gd` → `docs/img/s2/s2-feu-proche.png`, `s2-feu-large.png`, `s2-ruines.png`.
- La capture a révélé un défaut invisible au test headless : flammes et fumée affichées en quads de 1 m
  (`billboard_keep_scale` manquant, l'échelle `size_m` était perdue) et émises dans le volume des maisons.
  Corrigé : `billboard_keep_scale`, `flame_height_m` 4,5 → 7,5, flammes en mélange alpha (l'additif virait
  au crème en plein jour), flammes et fumée plus denses (`data/fx/siege_fire.json`).

## Limites connues à traiter plus tard
- S1 : porte de Guyenne masquée par ses tours (géométrie antérieure) ; pas de son d'effondrement.
- S2 : les ruines restent frustes (toit aplati sur un socle noir) ; le HUD de la capture affiche encore
  « Déploiement » (artefact du script, bataille démarrée par l'API).
- S2 : ~~règles embarquées par `include_str!`~~ (RS-F : lues depuis `data/` au chargement, ADR 0099) ; l'IA n'évite pas les rues en feu
  et n'utilise `burn` que pour les faubourgs ; ~~pas de bouton « incendier »~~ (RS-F : bouton de la barre des ordres, touche I) ; l'église ne s'effondre pas
  visuellement ; pas de lutte contre le feu.
