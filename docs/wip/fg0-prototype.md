# FG0 — Prototype et planche de style (figurines fines)

Branche : `feat/fg0-prototype` (worktree agent). Plan : `docs/wip/fg-figurines-fines.md`.
Planche : `docs/img/fg/planche_fg0.png` (+ images séparées dans `docs/img/fg/`).

## État : terminé (prototype), en attente de validation du joueur
- [x] Base humaine : MakeHuman via MPFB 2.0.17 (données CC0, code GPL installé hors dépôt) ;
  `fg_makehuman_base.py` -> `makehuman_base/fg_base_male.blend` (0,8 Mo).
- [x] Ajustement au rig `human` : membres joint à joint (clavicules, bras, jambes), tronc/cou/tête
  en similitude (échelle 0,904, hanches calées) ; poids MPFB `game_engine` renommés vers les os
  Quaternius + lissage des épaules ; poings fermés à la pose de liaison. Testé marche, estoc,
  arc long, mort, corps nu puis habillé : déformation propre.
- [x] Cheval : « Rigged Horse » (Lyndon Daniels, OpenGameArt, CC0, textures 2k) : similitude,
  largeur ramenée à celle du cheval Quaternius (0,72), encolure relevée (25°), déformation RBF
  sur 12 repères (articulations des jambes, garrot, croupe), poids par chaleur des os (copie du
  squelette aux os prolongés), crins par report barycentrique. Testé pas, galop, mort.
- [x] Homme d'armes : coques du corps (haubert, surcot, chausses), jupe fendue, bassinet,
  camail drapé (lancer de rayons sur le haubert), épée XVI, écu, ceinture, fourreau, souliers.
- [x] Chevalier monté : même corps sur l'armature cavalier de `Mount`, lance (os virtuel `Prop`),
  caparaçon en deux pièces (séparé à la selle), étriers élargis de 6 cm.
- [x] Cuisson test (Cycles) : atlas 2048 normal (mailles 9 mm, tissage + plis, cuir, acier
  martelé, peau), ORM (AO, rugosité, métal), masque (livrée, armoiries, peau).
- [x] Planche + coûts.

## Commandes
```
blender -b --python tools/blender_scripts/fg_makehuman_base.py            # MPFB requis
blender -b --factory-startup --python tools/blender_scripts/battle_fine_proto.py -- <étape> --out <dir>
#   étapes : fit, horse, infantry, cavalry, current_infantry, current_cavalry
uv run --project tools python tools/blender_scripts/fg0_planche.py <dir>
```
Rien n'est écrit dans `game/assets/models/battle_skinned/` ; le pipeline actuel est intact.

## Coûts
| Figurine | LOD0 | LOD1 | LOD2 |
|---|---|---|---|
| infantry_0 actuel | 2 374 | 517 | ~230 |
| homme d'armes prototype | 12 291 | 2 400 | 499 |
| cavalry_0 actuel | 2 956 | 746 | ~370 |
| chevalier prototype (cavalier + cheval + caparaçon) | 16 440 | 2 796 | 713 |

Détail monté LOD0 : cheval 5 960 (corps 4 200, crinière 1 000, queue 600), cavalier 9 581,
caparaçon + selle ~900.

Textures (BC, mipmaps compris) : atlas figurine 2048² = normale BC5 5,3 Mo + ORM BC1 2,7 Mo
+ masque BC4 2,7 Mo ≈ 10,7 Mo ; en 1024² ≈ 2,7 Mo. Cheval : couleur + normale + AO 2k ≈ 10,7 Mo
(un seul jeu partagé par toutes les montures, robes par teinte du shader). 25 recettes × atlas
2048 propres = 270 Mo : trop. Recommandation : textures de détail tuilables partagées
(mailles, tissu, cuir, acier, 512², ~5 Mo) + petit atlas 1024 par famille (AO, masque, normale
des formes) ≈ 2,7 Mo × ~8 familles.

Impact attendu `--units=50` (11 800 soldats, estimation, à mesurer en FG5) : LOD0 < 24 m
(quelques centaines de soldats en gros plan) : +10 k triangles chacun ≈ +3 M ; LOD1 24-75 m
(~2 000 soldats) : +1,9 k chacun ≈ +3,8 M ; LOD2 (~8 000 + ombres) : +270 ≈ +2,2 M. Soit
jusqu'à ~+9 M triangles skinnés dans le pire cadrage, la charge sommet étant 4 os × 3 lectures
(×2 en interpolation). Pistes : LOD1 ≈ 1 500 (la silhouette tient, cf. vue à 30 m), relais
LOD0 à 16-20 m, LOD2 ≤ 300, ombres sur LOD2, supprimer les faces cachées sous le caparaçon.

## Écarts / limites
- Proportions : le rig `human` a des bras courts (humérus 18 cm) et une grosse tête ; le corps
  réaliste est comprimé aux bras (lisible sous la maille, visible nu). FG1 : envisager un rig
  `human` aux os allongés (mêmes os, mêmes clips, texture d'os re-cuite).
- Cheval : jarrets/paturons tordus au galop (poids par chaleur à retoucher), tête un peu
  déformée par le relèvement d'encolure, yeux mi-clos. Les étriers élargis imposent de
  re-cuire la texture d'os `cavalry` (mêmes os, mêmes clips).
- Camail trop sombre (AO), bassinet lisse (pas de martelage visible), plis du surcot faibles :
  réglages de cuisson/matière à affiner en FG2/FG3.
- Rendu « actuel » en Blender : couleurs par sommet sans les motifs procéduraux du shader du
  jeu (mailles, tissage) — l'actuel est donc un peu désavantagé ; pas de capture Godot (disque).
- Le `.blend` du cheval (20 Mo) n'est pas versionné : téléchargé à la demande.
- Masque de livrée cuit mais pas encore exploité par un shader (FG3, après DA1).

## Recommandation FG1 / FG4
- FG1 : garder MakeHuman/MPFB + la méthode (membres joint à joint, poids MPFB renommés) ;
  6-8 visages par cibles MPFB (fichier `DETAILS`), cheveux/barbes à ajouter ; budget LOD0
  ~10-12 k à pied, LOD1 1,5-2,4 k ; exporter en `CAM1` via `export_mesh` (inchangé).
- FG4 : cheval OGA retenu ; reprendre les poids des jambes à la main ou par segments,
  re-cuire `cavalry` avec étriers élargis ; caparaçon en deux pièces + chanfrein.

## Prochaine étape
Validation de la planche par le joueur, puis FG1.
