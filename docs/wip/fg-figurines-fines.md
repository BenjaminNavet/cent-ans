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
| FG2 | Équipement fin : mailles, plates, casques, armes, surcots avec plis, par recette (28 recettes) ; allonge du tir à l'arc | FG1 | lancé 26/09 |
| FG3 | Matières cuites : atlas normal + ORM + masque de livrée, intégration shader skinné | FG1, **après DA1** (même shader) | à faire |
| FG4 | Cheval en production : poids des jambes (galop), étriers élargis, chanfrein, harnachement, caparaçon, chaîne LOD | FG0 | lancé 26/09 |
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
