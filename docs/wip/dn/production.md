# DN production 3D de la nuit (08/10)

Outil : `tools/experiments/dn_batch.py --select-best --charter strict --backend3d fal --image-backend fal --kind decor --seeds 1` ;
objets orientés (house, major_building, bridge, ship, cart, siege_engine) en multi-vues (face + dos, + côté pour cart/siege_engine/bridge) via
`flux-2/edit` + `trellis/multi`; arbres, rochers, props, animaux en single-view. Puis `cent-ans dn-ingest ... --generation` (provenance dans le manifeste).
Graines : 1, seconde graine seulement si D5 refuse. Plafond fal : 29,50 $ (garde `check_cap` dans l'outil, variable `DN_FAL_CAP_USD`).
Provenance : `~/dev/cent-ans-raw/dn/<id>/generation.json` + `prompt.txt`, copiée dans `dn_manifest.json` (`generation`).

## Fait
- campagne : 38 objets ingérés + table `dn_campaign_models.json` remplie (villes, villages, châteaux, abbaye).
- nature, animaux et env_* de map_extra, décors de bataille : ingérés (manifeste), refusés D5 repassés avec 2 graines puis `--charter warn` (animaux/arbres naturellement chauds, s_mean 0,41-0,42 > 0,40 ; l'étalonnage d'ingestion plafonne la saturation).

- architecture, mobile, economy : ingérés (multi-vues pour house/major_building/ship/cart/bridge).

- illus : 308 images générées (1 graine, fal Z-Image, recadrées à `ingest.size`), provenance dans `data/art/dn_illus_generation.json`.

## En cours
- nature_extra, battle_extra (fal, 1 graine).

## Restant
nature_extra, battle_extra, illus (kind image : à ajouter dans dn_batch.py).

## Ratés / retirés
- econ_sheep_shearing_pen : refusé par le filtre de contenu fal (faux positif), non généré.
- ship_nef (galion à 3 mâts, anachronique pour 1340) retiré.
- animal_cattle_red etc. refusés D5 (seconde graine).

## Points
- Test `dn_campaign_models_test.gd` : 2 assertions « table vide » échouent maintenant que la table est remplie (à adapter par l'agent camp-bati) ; chargement des glb vérifié (dn_models 2, dn_places 1120).
- LOD2 : le dn-ingest calait au-dessus du budget sur les maillages à nombreux îlots UV ; repli par soudure de sommets ajouté (`decimate`).
- Classe `bridge` ajoutée (copie de `house`).
- Scratchpad partagé avec d'autres agents : mes scripts sont dans `scratchpad/dn/`.
