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

## État : fait
- `TreeInst.role` (Forest/Open/Grove) ; `OpenThinning` (look.rs) appliqué dans `render::assemble` :
  arbres hors forêt amincis selon la distance caméra (plein jusqu'à 60, 6 % à 260), tous gardés à
  moins de 50 unités du foyer ; bosquets 50 % au loin, modulés par préréglage (low 0,3 ... high 1).
- `groves.rs` : bosquets (cellules 18 px, 25 %, 4-9 arbres, essences du rôle `isolated` du biome,
  une dominante) sur prairies/cultures hors forêt ; clairières (cellules 64 px, 50 %, rayon 3-5,5,
  contour ondulé, coeur de massif seulement) ; ressource `ForestClearings` (positions + `kind`
  d'objet de vie) pour un lot suivant qui placera l'objet (`countryside/` ou `life/`, non touchés).
- Les modèles Meshy `grove_*` ne sont PAS utilisés (mailles de 20 m sans ligne d'imposteur).
- Dump `vegetation.open_forest` (par palier 60/150/300/600/inf : total et gardés, forêt/hors forêt/
  bosquets ; clairières) ; `vegetation.tiles.grove_trees`, `clearing_trees_removed`.
- Mesure (Orléans 2,10/47,98, d=250, 22 cases, avant réglage final) : hors forêt 14 118 -> 3 609
  (bande 300-600) ; forêt 90 213 -> 53 995 ; bosquets 4 289 -> 3 425. Réglage final plus serré
  (60/260, bosquets 25 %/50 % au loin) non re-mesuré.
## Restes
- Placer l'objet de vie dans les clairières (lecture de `ForestClearings`).
- Coût CPU de l'échantillonnage des bosquets non mesuré sur machine calme.
