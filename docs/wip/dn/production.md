# DN production 3D de la nuit (08/10)

Outil : `tools/experiments/dn_batch.py --select-best --charter strict --backend3d fal --image-backend fal --kind decor --seeds 1` ;
objets orientés (house, major_building, bridge, ship, cart, siege_engine) en multi-vues (face + dos, + côté pour cart/siege_engine/bridge) via
`flux-2/edit` + `trellis/multi`; arbres, rochers, props, animaux en single-view. Puis `cent-ans dn-ingest ... --generation` (provenance dans le manifeste).
Graines : 1, seconde graine seulement si D5 refuse. Plafond fal : 29,50 $ (garde `check_cap` dans l'outil, variable `DN_FAL_CAP_USD`).
Provenance : `~/dev/cent-ans-raw/dn/<id>/generation.json` + `prompt.txt`, copiée dans `dn_manifest.json` (`generation`).

## Fait
- TOTAL : 667 objets 3D ingérés (data/art/dn_manifest.json), 308 illustrations. Dépense fal ≈ 28,6 $ (plafond 29,50 $).
- campagne : 38 objets ingérés + table `dn_campaign_models.json` remplie (villes, villages, châteaux, abbaye).
- nature, animaux et env_* de map_extra, décors de bataille : ingérés (manifeste), refusés D5 repassés avec 2 graines puis `--charter warn` (animaux/arbres naturellement chauds, s_mean 0,41-0,42 > 0,40 ; l'étalonnage d'ingestion plafonne la saturation).

- architecture, mobile, economy : ingérés (multi-vues pour house/major_building/ship/cart/bridge).

- illus : 308 images générées (1 graine, fal Z-Image, recadrées à `ingest.size`), provenance dans `data/art/dn_illus_generation.json`.

## En cours
- (rien : tout est généré ; voir « Non générés »).

## Non générés (D5 refusé deux fois ou échec fal) — journalisés
Reprise locale (mflux, gratuit) : voir « Repris en local » ci-dessous et `docs/wip/dn/local-retry.md` (EN PAUSE).
- campaign : ship_nef
- map_extra : env_cliff_chalk_coast, env_autumn_oak_gold, env_harvest_sheaves
- architecture : house_andalus_courtyard, house_maghreb_dar
- mobile : ship_galley_genoese, ship_galley_aragonese, cart_relic_procession, pack_camel_laden, pack_camel_bactrian_laden
- economy : econ_saltpan_mediterranean
- nature_extra : tree_spruce_siberian, tree_carob, tree_oak_kermes, tree_orange_bitter, shrub_osier_coppice, env_fallen_log_forest_mossy, env_root_plate_windthrow, crop_wheat_ripe, crop_barley_ripe, crop_oats, crop_millet_steppe, crop_hemp_tall, crop_vine_stakes_row, tree_mulberry_silk, crop_rice_paddy
- battle_extra : camp_pitched_tent_pilgrim

## Restant
nature_extra, battle_extra, illus (kind image : à ajouter dans dn_batch.py).

## Ratés / retirés
- econ_sheep_shearing_pen : le filtre fal était un faux positif ; le modèle existe (`game/assets/models/dn/buildings/econ_sheep_shearing_pen_lod0.glb`), pas de reprise.
- ship_nef (galion à 3 mâts, anachronique pour 1340) retiré.
- animal_cattle_red etc. refusés D5 (seconde graine).

## Points
- Test `dn_campaign_models_test.gd` : 2 assertions « table vide » échouent maintenant que la table est remplie (à adapter par l'agent camp-bati) ; chargement des glb vérifié (dn_models 2, dn_places 1120).
- LOD2 : le dn-ingest calait au-dessus du budget sur les maillages à nombreux îlots UV ; repli par soudure de sommets ajouté (`decimate`).
- Classe `bridge` ajoutée (copie de `house`).
- Scratchpad partagé avec d'autres agents : mes scripts sont dans `scratchpad/dn/`.

## Repris en local (10/10, EN PAUSE)
Images Z-Image locales 3 graines faites pour 5 ids (choix = 1re graine, pas encore de 3D) :
cart_relic_procession, pack_camel_bactrian_laden, pack_camel_laden, ship_galley_aragonese, ship_galley_genoese
(S moyen 0,44-0,50 pour chariot/chameaux : encore hors charte stricte ; galères 0,18-0,31, conformes).
Les 22 autres restent non repris ; aucune 3D locale produite à ce stade.

## DN-RESTE (09/10)
Les 22 refusés D5, 5 ids sans 3D, 21 figures et 14 cartes ont été générés via fal (voir local-retry.md). « Non générés » ci-dessus est périmé, sauf ship_nef (retiré).
