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
- [x] Cœur (tours × 0,6, hauteur du donjon, basse-cour : puits + accessoires, communs ; test `castle_looks_like_a_castle`)
- [x] Rendu (donjon carré `_build_keep`, communs du kit ; test `nt8_castle_test.gd` vert)
- [x] Tests : cargo fmt, clippy -D warnings (sim-battle, godot-bridge), cargo test workspace, pytest (726 ciblés), smoke, sb_siege_bars_test, nt1_siege_shot (headless), nt8_castle_test ; capture `nt1_castle.png` régénérée (non lue)

## Prochaine étape
Lot terminé. Session principale : juger `docs/audit/captures/nt/nt1_castle.png`.

## Points ouverts
- Pas d'escalier vers une porte haute (porte au pied, pour garder l'emprise de la simulation).
- Tours du château : rayon (5 + fortification) × 0,6, soit 6 m au plus ; hauteur inchangée.
- Toit en pavillon sur tous les donjons (pas de variante en terrasse).
