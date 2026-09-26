# FG — Finesse des figurines de bataille (orchestration)

Demande du joueur (25/09) : « une meilleure définition des modèles d'unités ». Précisé : finesse
visuelle ; option B retenue (nouveau corps + matières cuites) ; tout gêne (visages et corps,
armures et matières, chevaux).

Direction : bible DA (`feat/da-direction-artistique`, `docs/design/2026-09-25-bible-da.md`),
**semi-réaliste façon Thrones of Britannia / Attila** ; planche de style validée sur pièce avant
toute production. Budget : 0 $ prévu (sources CC0, cuisson Blender) ; toute génération IA
éventuelle passe par la section DA de `docs/budget.md`.

## Constat (ADR 0014, lot UR1)
- Base Quaternius CC0 low-poly (~2 100-2 400 triangles à pied, 2 500-3 050 monté au LOD0).
- Aucune texture : couleur + code matière par sommet, matières procédurales dans
  `battle_soldier_skinned.gdshader` ; visages, mains, plis, rivets absents.
- LOD0 < 32 m, LOD1 < 75 m, LOD2 au-delà, imposteurs > 300 m : un LOD0 plus riche coûte peu.
- Animations en texture d'os par rig (`human`, `cavalry`) : garder les **mêmes os** permet de
  réutiliser tous les clips et poses calculées.

## Lots

| Lot | Objet | Dépend de | État |
|---|---|---|---|
| FG0 | Prototype + planche de style : homme d'armes et cheval nouvelle définition, rendus Blender côte à côte avec l'actuel ; choix de la base (MakeHuman/MPFB CC0 ou autre), méthode d'ajustement au squelette Quaternius, budget de triangles | — | fait, validé par le joueur, fusionné (8dfc13d1) |
| FG1 | Corps humain en production : rigs `human` et cavalier (`R:`) aux bras allongés (texture d'os recuite, mêmes clips), 6-8 visages, chaîne LOD, sortie CAM1 lisible par le shader actuel, derrière un drapeau | FG0 | fait, fusionné (4522c2ad) |
| FG2 | Équipement fin : mailles, plates, casques, armes, surcots avec plis, par recette (28 recettes) ; allonge du tir à l'arc | FG1 | fait, fusionné (bda81191) |
| FG3 | Matières cuites : atlas normal + ORM + masque de livrée, intégration shader skinné | FG1, **après DA1** (même shader) | **en pause** : cuisson faite, vérifs/captures/ADR restantes (branche `feat/fg3-materials`) |
| FG4 | Cheval en production : poids des jambes (galop), étriers élargis, chanfrein, harnachement, caparaçon, chaîne LOD | FG0 | fait, fusionné (0a53488e) |
| FG5 | Performance A/B (`--units=50`, Ultra), captures, ADR (relève le plafond de triangles de la bible § 6) | FG1-FG4 | à faire |

Coordination : DA1 (armoiries sur les figurines, en pause dans une autre session) touche le même
shader et les mêmes recettes ; FG3 passe après sa fusion. FG0-FG2 et FG4 restent dans
`tools/blender_scripts/` et les binaires `battle_skinned/`.

Disque : 95 % plein le 25/09 (49 Go libres) ; un seul worktree par lot, supprimé après fusion.

## Pistes ultérieures
- Figurines scannées (scans de musée, image → 3D, Gaussian Splatting) et animations réalistes
  (capture, vidéo → mouvement) : analyse dans `docs/research/figurines-scan-et-animation.md`,
  à reprendre après FG1 (voie pragmatique pour FG2, animations en chantier séparé).

## Journal
- 25/09 : plan écrit ; FG0 lancé (agent en worktree).
- 26/09 : FG0 rendu. MakeHuman/MPFB 2.0.17 (CC0) ajusté au rig `human` ; cheval « Rigged Horse »
  OpenGameArt (CC0) sur le rig `cavalry`. LOD0 12,3 k (pied) / 16,4 k (monté). Planche
  `docs/img/fg/planche_fg0.png` sur la branche. Défauts connus : galop (jarrets tordus), bras
  comprimés (humérus courts du rig), camail sombre. Mémoire : un atlas 2048 par recette ×25 = trop
  (270 Mo) → textures de détail partagées + atlas 1024 par famille. Détails : `docs/wip/fg0-prototype.md`.
- 26/09 : joueur : « oui pour les deux » (style validé, bras allongés) puis « ne valide plus par
  moi, vas-y » → l'orchestrateur enchaîne FG1-FG5 sans nouvelle validation. FG0 fusionné
  (8dfc13d1). Vague 1 : FG1 et FG4 en parallèle. Partage : FG1 possède les longueurs d'os
  humains (rig `human` ET os `R:` du cavalier) ; FG4 possède les os du cheval et les étriers.
  Les deux recuisent `cavalry.bones.bin` : conflit binaire résolu par relance du script après
  fusion (FG1 fusionné d'abord).
- 26/09 : FG1 fusionné (4522c2ad). 28 recettes sur corps MakeHuman sous `--fine-figures`
  (`game/assets/models/battle_fine/`, point d'entrée `tools/blender_scripts/battle_fine.py rigs|figures|check`).
  Humérus 0,176 → 0,251 m. `--units=50` : primitives +11 %, mais i/s 27-30 → 18-25 (sommets ×4 au
  LOD0, ×3,5 au LOD1) → FG5 : LOD0 relayé plus tôt, LOD1 plus léger. Montés : cheval Quaternius en
  attendant FG4 (raccord dans `FineMount`, `battle_fine.py`). Arc : allonge 0,55 m au lieu de 0,68.
  FG2 lancé (équipement), sans toucher cheval/harnachement (FG4).
- 26/09 : FG4 fusionné (0a53488e). Cheval CC0 relevé articulation par articulation (`OGA_JOINTS`),
  galop propre ; destrier / roncin / genet ; selle, bride, chanfrein, flançois, caparaçon ; robes par
  teinte du shader × `fg_shade`. `STIRRUP_HALF_WIDTH` 0,36 m aussi dans le rendu par défaut (un peu
  d'air botte/flanc sur le cheval Quaternius, accepté : remplacé en FG5). Monté LOD0 14-18 k.
  FG2 devra fusionner main et relancer `battle_fine.py -- figures` pour les 8 montés ;
  `battle_skinned.export_mesh` lit `fg_shade` (conflit possible).
- 26/09 : FG2 fusionné (bda81191). Casques (bassinets, museau de chien, heaume, salades, chapel,
  cervelière), harnois début/tardif, jaque, brigandine, plis ; armes et écus refaits ; UV par pièce.
  LOD0 9,4-11,9 k à pied, 15,2-17,4 k monté ; primitives +12 %, i/s non significatif. Arc : tirage
  0,59 m, flèche raccourcie. Barbes FG1 trop sombres → FG3. DA1 constaté fusionné dans main → FG3 lancé.
- 26/09 : **PAUSE demandée par le joueur** (reprise dans une autre session). État :
  - FG0, FG1, FG2, FG4 fusionnés dans main (dernier : bda81191). Rendu fin sous `--fine-figures`.
  - FG3 en pause, non fusionné : branche `feat/fg3-materials` (commit 672930e2), worktree
    `.claude/worktrees/agent-aba08b13133c215dc` (cache `.godot` supprimé). Fait : cuisson des 28
    recettes (atlas 512² LOD0 / 256² LOD1 : normale de forme, occlusion, masque barbe/cheveux/
    martelage), 8 tuiles de détail partagées, une texture cheval 1024², variante shader
    `FG3_BAKED` seulement sous `--fine-figures` (défaut identique au pixel, vivants et cadavres),
    LOD2 sans texture, 15,7 Mo BC7. Camail noir corrigé (faces inversées depuis FG0), barbes OK.
    Reste : captures `fg3_*.png`, smoke + captures DA1 (`da1_arms_shot.gd`) et EP12
    (`ep12_shot.gd`) avec/sans drapeau, banc `--units=50`, réglages martelage/bois, ADR (~0088),
    `battle_fine/SOURCE.md`. Détails et commandes : `docs/wip/fg3-matieres.md` sur la branche.
  - **Reprise** : ouvrir le worktree FG3 (ou relancer un agent dessus), finir les vérifs, fusionner
    main dans la branche puis ff-only dans main ; ensuite FG5 (perf : LOD0 relayé plus tôt,
    LOD1 plus léger ; bascule du rendu fin par défaut ; ADR relevant le plafond de triangles de la
    bible § 6) ; nettoyer le worktree FG3 après fusion.
