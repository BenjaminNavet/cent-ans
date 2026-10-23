# 0742 — Cache du sol par nœud de relief

Date : 2026-10-23 · Chantier camp60 (60 i/s en carte de campagne)

## Contexte

Le shader du sol de la carte (`terrain.wesl`, 2 400 lignes) calcule à chaque pixel et à chaque
image un « bloc régional » : couches de sol texturées, champs peints, parcelles, détail rapproché,
sous-bois et canopée. Mesures sur M4 Pro, réglages du joueur (Ultra, Retina plein écran, mise à
l'échelle automatique ≈ 1080p rendus), panoramique :

| variante | d = 30 | d = 150 | d = 500 |
|---|---|---|---|
| sol complet | 45,5 ms | 32,5 ms | 27,9 ms |
| sans bloc régional (`tnoproc`) | 25,7 ms | — | — |
| sol plat (couleur seule) | 20,0 ms | 7,4 ms | 7,7 ms |

Ce bloc ne dépend que du point de la carte (altitude, pente, tables de sol, saison) : ni de la
caméra, ni de la lumière, ni du temps. Le recalculer 60 fois par seconde est du travail perdu.

## Décision

Chaque nœud du quadtree de relief (64 ou 32 quads) reçoit une couche d'un atlas
`texture_2d_array` (N × N texels sur le carré du nœud) où le bloc régional est **cuit une fois** :
albédo + part de forêt (Rgba8UnormSrgb), décalage de normale (Rg16Float).

- `regional_ground` (`terrain.wesl`) regroupe le bloc ; le fragment direct et la cuisson
  l'appellent tous deux. L'empreinte au sol de la cuisson est la taille d'un texel ; comme la
  densité des nœuds suit déjà la distance (3 à 6 px par quad), un texel couvre ≈ 1 pixel.
- La couche et l'uv du nœud voyagent dans l'attribut `morph` (`z` couche, `w` uv empaqueté
  exactement, `mesh.rs::pack_cache_uv`).
- Cuisson (`ground_cache.rs`) : passe du monde de rendu, dans le `Core3d` de la caméra de campagne
  (marqueur `GroundCacheView`), avant la passe principale. Elle dessine le maillage du nœud à plat
  (`bake_vertex` / `bake_fragment` du même module) dans une cible de travail puis la copie dans la
  couche (une cible liée à la fois comme texture et comme attache est interdite). Groupe 3 = bind
  group du matériau du sol, groupes 0-2 vides ; les définitions de shader viennent de
  `MeshPipeline::specialize` (le pipeline principal n'existe pas tant qu'aucun nœud n'est visible).
- Un nœud ne s'affiche qu'après sa première cuisson ; le mécanisme `retiring` garde l'ancien
  nœud jusque-là. Au plus 6 cuissons par image tant qu'un nœud attend, 2 sinon.
- Le pipeline principal, compilé avec `GROUND_CACHE` (clé `bind_group_data` du matériau), lit la
  couche (`textureSampleLevel`, sans dérivées) au lieu du bloc régional.
- Invalidation : signature des entrées du bloc (carte, saison quantifiée au 1/64, sol, champs,
  détail, poignées de textures) ; un changement recuit tous les nœuds, au plus une fois par
  seconde.
- Préréglage : clé `map_ground_cache` (texels par côté, 0 = calcul direct) ; essai :
  `CENT_ANS_GROUND_CACHE=<texels>`. Le choix est pris à l'entrée en campagne.

## Conséquences

Mesures (réglages du joueur, panoramique, p50 / p95 en ms) :

| distance | sans cache | cache 256 | cache 320 |
|---|---|---|---|
| 30 | 47,5 / 51,0 | 27,2 / 30,9 | 28,2 / 31,4 |
| 150 | 33,9 / 36,2 | 14,6 / 24-28 | 13,6-34 (variable) |
| 500 | 34,1 / 37,1 | 13,8 / 21,4 | 13,6 / 21,2 |

À d = 150, trois alternances avec / sans cache donnent toutes 14,6 contre 33,9. Une cuisson coûte
≈ 0,15 ms (recuisson forcée de 8 nœuds par image : +1,3 ms) ; pendant un panoramique le quadtree
en demande ≈ 6 par image (nœuds nouveaux et pages plus fines). Écart de rendu avec le calcul
direct : 0,8 / 255 en moyenne (320 texels, vue rapprochée). Préréglages : Ultra 320, Haute et
Moyenne 256, Basse 192, `legacy` 0.

- Mémoire : 384 couches × N² × 8 octets (N = 320 : 315 Mo ; 256 : 201 Mo). Pic observé : 243 couches
  (panoramique à d = 150). Atlas plein : le nœud reçoit la couche -1 (sol faux), compté dans le dump
  (`ground_cache.exhausted`).
- Le détail est figé à l'empreinte d'un texel : en s'approchant d'un nœud, la finesse augmente
  par paliers (changement de niveau du quadtree), plus en continu.
- Le contenu d'une texture modifiée en place (même poignée) n'est pas détecté : à brancher sur
  `AssetEvent<Image>` si un tel cas apparaît.
- Les fonctions du bloc régional ne doivent pas lire `view`, `globals` ni `lights` (la cuisson n'a
  pas de vue) : à respecter en les modifiant.
