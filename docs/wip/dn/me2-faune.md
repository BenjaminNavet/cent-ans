# DN ME2 : troupeaux et faune (branche dn/me2-faune)

Etat : donnees + schema faits ; `fauna_layer.gd`, shader `fauna.gdshader`, test `game/tests/me2_fauna_test.gd` a ecrire.

## Conception
- `data/map/map_fauna.json` (schema `map_fauna.schema.json`) : especes = ids de `dn_catalog_map_extra.json`, zones = ellipses lon/lat (EPSG:3035 via map.json), densite en troupeaux / 1000 km2, filtres de biome (splat), pente, altitude, proximite mer/fleuve, saisons (transhumance : profils de poids par saison).
- Modele : `game/assets/models/dn/fauna/<id>_lod{0,1,2}.glb` (classe d'ingestion `animal`), sinon `placeholder` (folk/horse|cow|sheep.glb), sinon maillage procedural.
- Rendu : un MultiMesh par espece et par cellule (96 px), cellules construites a la demande autour du point vise ; errance, pas et rebond dans le vertex shader (pas d'armature) ; taille tenue a l'ecran (k x d^0.8) ; visible sous max_distance (< vue parchemin).
- Brancher un nouveau modele : mettre le glb ingere, ou editer une entree `species`.

## Prochaine etape
Ecrire la couche, la brancher dans `CampaignLife`, test headless, mesure.
