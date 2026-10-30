# 0826 : sol « photo aérienne » de la campagne (VIS-L1)

Statut : accepté (lot VIS-L1, boucle juge 2, itération 3). Amende 0742 (le cache de sol reste, une clipmap n'est pas construite),
0814 (fondu des lanières par largeur écran) et 0710 (désaturation aérienne réduite, saturation d'affichage relevée).

## Contexte

Le juge visuel frais du 30/10 (`docs/audits/2026-10-30-juge-visuel-iter3.md`, défaut G1-1) relève : sol « peigné » à d 150-400,
lanières en aplats à d 25, couture rectangulaire en Normandie, saturation du sol plafonnée à 0,20-0,24 dans toutes les vues 3D
(bible : plaine 0,56), forêts presque aussi claires que les prés (v forêt / v pré 0,65-0,70, cible 0,48), teinte menthe en Normandie
(117-123°, bible 89°).

Constats de lecture avant de coder (qui amendent la spec du lot) :

- Le sol n'est **pas** recalculé par fragment et par image : l'ADR 0742 le cuit déjà par nœud de relief (atlas `texture_2d_array`,
  un texel ≈ un pixel), le fragment principal ne fait que le lire. Une clipmap centrée caméra doublerait ce mécanisme sans rien
  régler : le peigné vient du **contenu** cuit, pas de la cadence de calcul.
- Le peigné est le patchwork lointain (`hb_far_patch`) et les parcelles du bloc proche de `fields_apply` : lanières 1:4 à 1:9 dont le
  petit côté tombe sous 2-3 px, échantillonnées une fois par texel, sans préfiltrage.
- La carte de couleur est déjà à s 0,52, v 0,43, h 84° ; la saturation est perdue dans la chaîne de rendu (lumière froide, AgX,
  désaturation aérienne, grade) : saturer l'albédo seul fait dériver la teinte vers le cyan (menthe), mesuré sur 3 essais.

## Décision

1. **Découpage de `terrain.wesl`** (2 970 lignes, désormais 365) en modules importés : `terrain_params` (blocs uniformes, liaisons,
   constantes communes), `noise`, `snow` (neige, étages de montagne VIS-R4, roche, glaciers : code déplacé tel quel), `parchment`,
   `water_edge`, `atmosphere`, `seasons`, `ground_layers`, `parcels`, `fields_paint`, `ground_look`, `ground_albedo`. Les points
   d'entrée (`vertex`, `fragment`, `bake_*`) et les constantes de creusement des rivières (testées par le côté eau) restent dans
   `terrain.wesl`. Premier commit : déplacement pur, `shader_check` vert.
2. **Pas de clipmap** : le cache par nœud (0742) reste le bake. Le préfiltrage « gratuit par mips » est remplacé par un préfiltrage
   analytique, moins cher que quatre niveaux de 2048² (64 Mo) et sans toucher au LOD.
3. **Préfiltre des lanières par largeur écran** (`view.strip_px` = [1, 12] px de `data/art/ground_biome_mix.json`). `hb_far_patch`
   choisit, par largeur écran du petit côté de la cellule, entre trois tailles de cellules (1, 1/z, 1/z² ; z = `terroir_scale`), puis
   la carte de couleur ; il renvoie la couleur et un poids. Le bloc proche de `fields_apply` fond de la même façon les parcelles trop
   étroites dans la couleur du sol. Plage large (1 à 12 px) : la cuisson donne à deux nœuds voisins une empreinte de texel double ;
   un fondu étroit transformait ce doublement en couture. `strip_px = [0, 0]` rend l'ancien fondu par empreinte.
4. **Couleur du sol en données** (`view.ground_look`, module `ground_look.wesl`, appliqué à l'albédo cuit, une fois par texel) :
   gain de chroma modéré (1,25), rotation de teinte vers la cible (66° sur l'albédo ; l'affichage tombe vers 85°), tirée plus fort
   (0,95) pour les teintes plus bleues que la cible (la menthe) que pour les plus jaunes (0,35, qui garde la variété des cultures),
   valeur des forêts (0,55) et des terres ouvertes (0,9), hors mer et hors montagne (bande d'altitude agricole des règles géo).
   Gains de valeur du labour (1,5) et du chaume (1,25) pour des parcelles claires lisibles (bible : labour 32°, v 0,62).
5. **Affichage** : `aerial.desaturate` 0,4 → 0,15 (spec) ; `legacy_adjust.saturation` 1,16 → 2,8 (schéma relevé à 4). C'est le
   seul levier qui restitue de la chroma sans dérive de teinte, AgX en retirant près des deux tiers ; il touche aussi la mer
   (bible : s 0,73), ce qui va dans le sens voulu. La montagne (`mountain_stages`) n'est pas modifiée.
6. **A/B** : `CENT_ANS_PERF_OFF=parcel` (ADR 0814) coupe aussi `strip_px` et `ground_look` (mêmes binaire, même données).

## Conséquences

- `FieldParams` grandit de 3 vec4 (`look_a`, `look_b`, `look_c`), aucun échantillonneur, aucune texture (limite Metal de 16 respectée).
- Le cache de sol est recuit au changement des données (signature du matériau).
- Textures de culture Z-Image (sillons, chaume, haie bocagère) : non faites dans ce lot. Les 43 matériaux du pack
  `tx_campaign_parcels` (labour, chaume, blé…) sont conservés ; le labour et le chaume sont éclaircis par gain de données. À reprendre
  si le juge trouve les lanières trop lisses à d 25.
- Couture de tuile : supprimée à l'œil et à la mesure (saut de teinte entre bandes 12° → 1,8° en Normandie), sans garantie
  théorique : tout contenu dépendant de l'empreinte garde un risque de saut entre nœuds de niveaux différents (limite connue du cache
  par nœud).
- La saturation d'affichage relevée agit sur toute la scène du monde (bannières déjà vives restent bornées par le clamp).

## Mesures (captures 1280×800 hors écran, `tools/compare_views.sh`, scripts de mesure hors dépôt)

| Vue | avant h / s / v plaine | après h / s / v plaine | avant → après autre |
|---|---|---|---|
| camp_0600 | 85° / 0,22 / 0,48 | 74° / 0,47 / 0,64 | v forêt / v pré 0,68 → 0,63 |
| camp_0200 | 94° / 0,21 / 0,42 | 82° / 0,45 / 0,53 | v forêt / v pré 0,65 → 0,56 ; énergie passe-bande 4-12 px du sol (boîte fixe) 2,66 → 0,89 (÷ 3) |
| camp_0060 | 101° / 0,22 / 0,40 | 84° / 0,45 / 0,47 | labour clair (v ≥ 0,5) 3,1 % → 15,2 % |
| camp_0025 | 99° / 0,22 / 0,38 | 86° / 0,45 / 0,42 | labour clair 3,8 % → 31,5 % |
| vis_r3b_normandie | 117° / 0,22 / 0,47 | 84° / 0,42 / 0,58 | saut de teinte max entre bandes 12° → 1,8° |

Critères du lot : plaine h 80-95° et s ≥ 0,42 tenus à d 200, 60, 25 et en Normandie ; à d 600 la teinte est de 74° (jaune-vert, haze) et la
valeur de 0,64 (cible 0,50). Labour clair ≥ 15 % à d 60 : 15,2 %. Couture : saut de teinte entre bandes < 5° en Normandie. Énergie
directionnelle ÷ 3 sur le sol de camp_0200. v forêt / v pré ≤ 0,6 : tenu seulement à d 200 (0,56) ; la mesure par percentiles de
pixels vert écarte les forêts les plus sombres (v < 0,15), un essai à `forest_value` 0,4 ne l'a pas fait bouger ; à relire par le juge.
Non mesuré : coût GPU (aucun profilage lancé dans le lot, règle de l'agent) ; les modules ne changent pas le coût, le préfiltre ajoute
au plus un niveau de cellules à la cuisson.
