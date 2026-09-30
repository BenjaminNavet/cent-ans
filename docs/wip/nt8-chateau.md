# NT8 — Rendu du château de siège

Chantier NT (`docs/wip/nt.md`), suite de NT1 (ADR 0126, `docs/wip/nt1-sieges.md`).
Branche : `feat/nt8-castle-look`. Cargo : target privé `core/target-nt8`, `profile.dev.debug=0`.

## Objectif
Capture `docs/audit/captures/nt/nt1_castle.png` : tours du château énormes, donjon = tour ronde
agrandie, basse-cour vide. Viser un château du XIVe (Vincennes, Largoët, Najac).

## Plan
- Données (`places.castle` de `data/rules/siege_town.json` + schéma) : `tower_radius_scale`,
  `keep_height_above_wall_m`, `bailey_props`, `bailey_kinds`, `lean_to_kinds` ; `buildings` 5-8.
- Cœur : rayon des tours du château × facteur ; `House.height` du donjon ; mobilier de la
  basse-cour (puits + accessoires, pas d'étals de marché) ; accessoires des communs.
- Pont : `houses[].height`.
- Rendu (`battle_siege.gd`) : donjon carré (talus, mâchicoulis, créneaux, échauguettes, toit
  en pavillon d'ardoise) ; communs du château = granges / longères / maisons de pierre du kit.
- Test Godot `game/tests/nt8_castle_test.gd` : un donjon, plus haut que les tours, tours ≤ seuil.

## État
- [x] Squelette (données, schéma, champs Rust, pont)
- [ ] Cœur
- [ ] Rendu
- [ ] Tests, capture

## Prochaine étape
Implémenter le cœur (siege_layouts.rs, props.rs).
