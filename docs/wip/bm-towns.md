# BM towns : villes détaillées au zoom proche (client Bevy)

Lot `client/src/campaign/towns/` (branche bm/towns).

## État
- Fait (pur, testé) : `data.rs` (towns_1340.json, règles de plan/murs, croissance), `kit.rs` (manifest du kit glTF, choix par proportions), `plan.rs` (port de `town_plan.gd` : rues drapées, îlots, parcelles, marché, paroisses, monuments, murailles, tours, portes, pont, sol), `growth.rs` (port de `town_growth.gd`), `profile.rs` (`town_render.tres`), `rng.rs`.
- Données : `data/art/town_look.json` + schéma (teintes, rubans, mobilier de place).

## Fait (vague 4)
- `layer.rs` : flux, plan, blocs / kit, rubans, murs, monuments, mobilier de place, croissance sim, fondu, `GroundHeight`, dump `campaign_towns`. Preuve : run_bg Abbeville (6 villes, 1743 maisons, 3167 blocs ; mode detail : 1792 modèles du kit, 0 erreur ; croissance forcée 3 quartiers + pierre : 252 maisons, 347 pièces).
- Pas de capture (règle commune) : aspect des blocs, teintes et soulèvement des rubans non vérifiés à l'oeil.

## Restes
- Villes emblématiques 1:1 (`landmark_plan.gd`, `landmark_monuments.gd`), `outbuilding_layer.gd` (hameaux, moulins hors les murs), styles régionaux du kit (`building_regions.json`, `BuildingMaterials`), arbres des jardins, pont, toits en prisme, ombres, plan sur fil de travail, fondu de la maquette côté markers (lire `TownsState`).
