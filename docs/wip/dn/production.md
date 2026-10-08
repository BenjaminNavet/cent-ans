# DN production 3D de la nuit (08/10)

Outil : `tools/experiments/dn_batch.py --select-best --charter strict --backend3d fal --image-backend fal --kind decor --seeds 1` ;
objets orientés (house, major_building, bridge, ship, cart, siege_engine) en multi-vues (face + dos, + côté pour cart/siege_engine/bridge) via
`flux-2/edit` + `trellis/multi`; arbres, rochers, props, animaux en single-view. Puis `cent-ans dn-ingest ... --generation` (provenance dans le manifeste).
Graines : 1, seconde graine seulement si D5 refuse. Plafond fal : 29,50 $ (garde `check_cap` dans l'outil, variable `DN_FAL_CAP_USD`).
Provenance : `~/dev/cent-ans-raw/dn/<id>/generation.json` + `prompt.txt`, copiée dans `dn_manifest.json` (`generation`).

## Fait
- campagne : 38 objets ingérés + table `dn_campaign_models.json` remplie (villes, villages, châteaux, abbaye).
- nature (1re graine) et animaux de map_extra (1re graine) ingérés ; secondes graines en cours pour les refusés D5.

## En cours
- env_* de map_extra, battle (décors), secondes graines nature / animaux.

## Restant
architecture, mobile, economy, nature_extra, battle_extra, illus (kind image : à ajouter dans dn_batch.py).

## Ratés / retirés
- ship_nef (galion à 3 mâts, anachronique pour 1340) retiré.
- animal_cattle_red etc. refusés D5 (seconde graine).

## Points
- Test `dn_campaign_models_test.gd` : 2 assertions « table vide » échouent maintenant que la table est remplie (à adapter par l'agent camp-bati) ; chargement des glb vérifié (dn_models 2, dn_places 1120).
- LOD2 : le dn-ingest calait au-dessus du budget sur les maillages à nombreux îlots UV ; repli par soudure de sommets ajouté (`decimate`).
- Classe `bridge` ajoutée (copie de `house`).
- Scratchpad partagé avec d'autres agents : mes scripts sont dans `scratchpad/dn/`.
