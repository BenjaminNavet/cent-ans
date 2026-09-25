# 0024 — Imposteurs lointains des figurines, cuits en jeu depuis les figurines V2

Date : 2026-09-25 (lot BV3, « bataille vivante » 3 ; complète les ADR 0014 et 0016)

## Contexte
La taille d'unité Ultra (×2,5, ADR 0016) double les primitives (BV1 : 3,8 M sur `--units=50`)
et coûte 20 à 25 % d'images par seconde. Au-delà de 300 m, une figurine fait 6 à 7 pixels de
haut en 1600 × 900 : le LOD2 (230 à 430 triangles) y est du gaspillage. L'ADR 0014 prévoyait
des imposteurs « cuits dans le pipeline Blender » (atlas de 8 angles × quelques images).

## Options
- **A. Atlas cuit dans Blender** : un atlas par figurine, dans le dépôt. La livrée et le blason
  varient par faction et sont calculés par le shader : il faudrait cuire un masque de livrée et
  refaire les teintes, ou un atlas par faction (des dizaines de textures).
- **B. Atlas cuit en jeu au début de la bataille** : on rend les figurines V2 elles-mêmes, avec
  le matériau du régiment (livrée, blason, éclaircissement lointain de A1-01), dans un
  `SubViewport` isolé, une fois par (camp, famille, variante) présents.
- **C. Carte d'octaèdre** (angles sur une demi-sphère) : utile pour des vues zénithales ; la
  caméra de bataille reste à 20-45° : 8 angles horizontaux suffisent.

## Décision
**Option B.** `BattleImpostors` (`game/scripts/battle/battle_impostors.gd`) :
- par clé `camp/famille/variante`, un `SubViewport` transparent (`own_world_3d`), caméra
  orthographique inclinée de 28°, lumière ambiante et une directionnelle douce ; une
  `MultiMesh` de figurines LOD0 en mode CUSTOM du shader skinné place chaque cellule :
  **8 colonnes** (la figurine tournée de c × 45°) × **16 lignes** (4 jeux : arrêt, marche,
  course / charge, action — tir pour les tireurs, mêlée sinon — × 4 images du cycle) ;
  cellules de 32 × 64 px à pied (1,3 × 2,6 m), 64 × 64 px montés (3,4 m) ;
- l'image est lue une fois rendue, mipmaps générées, puis le `SubViewport` est libéré
  (au plus 18 atlas de 256-512 × 1 024 px, cuits dans les premières images) ;
- `battle_impostor.gdshader` (`world_vertex_coords`) : un quadrilatère par soldat, tourné vers
  la caméra autour de la verticale ; colonne = angle de la caméra dans le repère de la
  figurine ; ligne = jeu de l'état du régiment + image selon l'horloge du régiment et la phase
  du soldat ; atlas prémultiplié (fond noir transparent : `rgb / alpha`) ; **alpha relevé selon
  le niveau de mipmap** (sinon les silhouettes, diluées dans le fond, passent sous le seuil et
  les régiments lointains disparaissent) ; normale fixée au fragment (pas de face arrière
  retournée) ;
- `BattleSoldiers._update_unit` : au-delà de **300 m** (distance caméra → centre du régiment,
  comme les autres LOD), la couche d'imposteurs remplace le LOD2 et reçoit **le même tampon
  d'instances** (12 flottants par figurine) : aucun travail GDScript par soldat.
- Actif pour toutes les tailles d'unité ; `--no-impostors` (ou `--no-bv3`) le coupe (A/B).

## Conséquences
- Ultra, `--units=50` (banc à 90 s) : primitives 3,79 M → 1,42 M ; mesures détaillées dans
  `docs/wip/bv3-finitions-bataille.md`.
- Au loin, une seule variante de casque par cellule (hachage de l'instance de cuisson) et pas de
  sang sur les imposteurs ; pas d'ombre (au-delà de 190 m, il n'y en avait déjà plus).
- Les figurines rigides (`--rigid-figures`) et les engins de siège n'ont pas d'imposteurs.
- Toute retouche des figurines V2 se retrouve automatiquement dans les imposteurs (pas d'atlas
  à régénérer dans le dépôt).
