# TS-T8 — bosquets et lecture des forêts (ADR 0710)

Branche `ts/t8`, worktree `/Users/jean_hubert/dev/gp-ts-t8`. Plan : `docs/wip/ts-style-total-war.md`.

## Plan
1. `TreeInst.role` (forêt / hors forêt / bosquet) posé par `edges.rs`, `groves.rs`.
2. `look.rs` + `campaign_forest_look.json` : blocs `open` (décroissance hors forêt avec la distance
   caméra, exemption près du foyer), `groves`, `clearings`.
3. `render::assemble` : amincit les arbres hors forêt selon `OpenThinning` (distance caméra / foyer).
4. `groves.rs` : bosquets compacts (CPU, par cellules globales) + clairières (trouées déterministes
   dans les massifs, positions exposées par la ressource `ForestClearings`).
5. Dump `vegetation.open_forest` : instances forêt / hors forêt / bosquets par palier de distance.

## État
- Fait (compile, 56 tests végétation verts) : rôles d'arbre, bloc `open`/`groves`/`clearings` dans
  `campaign_forest_look.json` + schéma, `OpenThinning` dans `assemble`, `groves.rs` (bosquets,
  clairières, ressource `ForestClearings`), dump `vegetation.open_forest`.
- Reste : dump en jeu (avant/après), réglage des parts, clippy/test/smoke finaux.
