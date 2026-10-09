# DN-TROUS : trous des maquettes de la carte

État : **terminé côté outil et ingestion** ; la régénération multi-vues des maquettes restantes attend du solde fal (voir « Pour une session locale »). Planche avant/après : hors dépôt, `…/scratchpad/planche_avant_apres.png`.

## Cause (trouvée par mesure, pas TRELLIS)
Les glb bruts TRELLIS sont étanches (score de la métrique 0,02 à 0,6) ; ce sont les LOD ingérés qui étaient troués (score médian 102 sur les 78 modèles de la table de campagne). Trois défauts dans `tools/blender_scripts/dn_ingest.py` :
1. `clean()` appelait `recalc_face_normals` : sur une scène de nombreuses coques (ville, cité, château) cela retourne des bâtiments entiers (15 à 42 % de la surface retournée sur Nuremberg, Kingston, Slesvig) ; le back-face culling donne alors murs déchirés, façades en lambeaux et intérieurs noirs. Désormais seulement pour une coque unique fermée.
2. Sommets de coutures UV non soudés : la décimation séparait les bords, le repli soudait jusqu'à 25 % de la diagonale (éclats flottants). Soudage exact (1e-5 de la diagonale) avant la décimation.
3. Classe `tree` : les cartes de feuillage (îlots < 1 %) étaient supprimées comme débris (couronnes perdues, flagrants de la revue). `island_min: 0` pour la classe.

## Métrique (`tools/experiments/dn_holes.py`, gratuite)
Après fusion des sommets par position : longueur des arêtes de bord / √surface, part de surface en composantes flottantes, part de faces très allongées, part de surface en coques retournées (volume signé, centré ; peu fiable sur coque ouverte). Calibrée sur les villes citées par le joueur :

| lieu cité | modèle | avant (LOD1) | après (LOD1) |
|---|---|---|---|
| Nuremberg | `city_west` | 110 | 0,09 |
| Kingston, Montargis | `town_west` | 118 | 1,7 |
| Slesvig | `city_nord` | 98 | 0,12 |

78 modèles utilisés par la campagne, tous troués avant (min 78, médiane 102) ; après : médiane 0,14, 6 au-dessus de 5. Restes (score LOD1 > 5, surtout des coques ouvertes mal jugées par la métrique, rendus corrects à l'œil) : `town_empire_south` 16,6, `abbey_balt`, `village_west`, `village_empire`, `castle_balt`, `castle_ital_rocca` (10 à 14).

## Route par maquette (consigne joueur : TRELLIS 1 multi-vues pour ville/cité/château/port)
- **Multi-vues TRELLIS 1** (face + dos + côté `flux-2/edit`, vues contrôlées en luminance, dos rendu et contrôlé) : `town_port_harbour` (nouveau, sous-famille `port`), `town_west`, `city_west`, `city_balt`, `city_byz`, `city_hungpol`, `city_isl`, `city_rus` (8). Aucune vue refusée : toutes passées au premier essai.
- **TRELLIS 1 une vue, ingestion corrigée** (solde fal épuisé avant le reste) : toutes les autres maquettes ; dos mesuré plus sombre que 0,6 × la face (« partie noire », gardées) : `city_ital_commune`, `city_med`, `city_steppe`, `city_isl_andalus`, `city_nord`, `castle_balt`, `castle_ital_rocca`, `castle_med`, `castle_isl_mamluk`, `town_byz`, `town_isl_anatolia`, `town_med`.
- TRELLIS 2 essayé sur 15 maquettes (ville, abbaye, village) puis abandonné : le maillage de 100 000 faces s'effondre au LOD1 (4 000 triangles) ; ces modèles sont repassés sur leur brut TRELLIS 1. TRELLIS 2 retenu pour les 13 objets isolés de la revue (voir `revue-assets.md`).
- Aucun modèle retiré : aucune maquette n'est repassée au polygone.

## Port
`town_port_harbour` (id fal 1339, sous-famille ajoutée dans `data/art/dn_campaign_models.json` : clé `port` de `table.town`, filtrée par `families` west/med, utilisée quand le lieu a `port: true` ; `DnCampaignModels.entries_for(…, port)`). Villes de type bourg et ports ouest/med seulement ; les cités portuaires gardent leur modèle de famille.

## Pour une session locale / quand le solde fal est rétabli
- Multi-vues restantes (≈ 0,07 $ chacune) : `city_iber_mudejar city_ital_commune city_ital_maritime city_med city_nord city_isl_mamluk city_steppe city_hansa city_isl_andalus castle_west castle_balt castle_byz castle_ital_rocca castle_med castle_iber town_iber town_med town_ital_towers town_byz town_empire_south town_hungpol` puis les autres châteaux/bourgs. Commande : `dn_batch.py <catalogue> --variant mv --backend3d fal --side-view --image-backend fal --until sheet --charter warn --no-local-fallback --reuse-image --only <ids>`, puis `dn_back_check.py`, puis `dn_reingest.py --real-manifest --raw-override '{"id": ".../mv/3d/fal__s<seed>.glb"}'`.
- Figures `archer_3_jack`, `fig_sled_driver_north` : images et glb multi-vues faits (`<id>/rv2/`), non cuits (chaîne `ga3_figures.py` sur la branche `dn/fig-bake`, non fusionnée).
- `env_glacier_ice_tongue` : reste de toit de chaume fusionné à gauche de la glace.
- Imposteurs d'arbres : relancer `ga3_vegetation_l2.py sheet/atlas` pour les arbres ré-ingérés (couronnes rétablies).
- `siege_cannon_early_1340` : ré-ingestion en échec (fichier hors lot, non touché).
