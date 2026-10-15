# BM towns : villes détaillées au zoom proche (client Bevy)

Lot `client/src/campaign/towns/` (branche bm/towns).

## État
- Fait (pur, testé) : `data.rs` (towns_1340.json, règles de plan/murs, croissance), `kit.rs` (manifest du kit glTF, choix par proportions), `plan.rs` (port de `town_plan.gd` : rues drapées, îlots, parcelles, marché, paroisses, monuments, murailles, tours, portes, pont, sol), `growth.rs` (port de `town_growth.gd`), `profile.rs` (`town_render.tres`), `rng.rs`.
- Données : `data/art/town_look.json` + schéma (teintes, rubans, mobilier de place).

## Prochaine étape
- `layer.rs` : flux des villes autour du rig, blocs / kit selon la distance, rubans de rues, murs, fondu `ZoomView`, hauteur `GroundHeight`, croissance depuis la sim, section de dump `campaign_towns`.

## Restes prévus
- Villes emblématiques 1:1 (`landmark_plan.gd`, `landmark_monuments.gd`), `outbuilding_layer.gd` (hameaux, moulins hors les murs), styles régionaux du kit (`building_regions.json`, `BuildingMaterials`), arbres des jardins, pont, ombres.
