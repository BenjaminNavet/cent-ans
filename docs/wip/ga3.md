# GA3 — Sondes image-vers-3D (fal.ai TRELLIS)

Branche `feat/ga3`, worktree `../game_project-ga3`. Clé `FAL_KEY` dans l'environnement (vérifiée 30/09).
Plan d'origine : `docs/wip/ga.md` §GA3. Budget GA3 ≤ 8 $ (sondes < 1 $). Dépenses → `docs/budget.md`.
Ne pas toucher `feat/sr` (autre session, figurines semi-réalistes).
Brutes (hors dépôt) : `~/dev/cent-ans-raw/ga3/` ; références figurines : `~/dev/cent-ans-raw/sr3/*.png` (1K, face/profil/dos).

## Consigne de reprise
> Lis ce fichier et `git log --oneline feat/ga3 -15`, continue à la première case non cochée.

## Sondes (go/no-go du joueur à la fin)
- [x] S1 décor : maison à colombages → TRELLIS → `tools/blender_scripts/ga3_cleanup.py`
      (décimation, LOD0/1/2, cuisson albédo 1024²) → `game/assets/models/props_ga/` ; planche
      `docs/img/ga3/s1_house.jpg`.
- [x] S2 figurine : longbowman (`sr3/longbowman.png`, vues multiples) → TRELLIS → nettoyage →
      rattachement au squelette de bataille existant (poids automatiques) → rendu marche + tir ;
      planche `docs/img/ga3/s2_archer.jpg` ; verdict technique (déformations, triangles, coût).
- [x] S3 comparatif générateurs figurine (2,32 $) : Tripo H3.1 multivue / Meshy 7.1 multi-image
      (rig + anim) / TRELLIS 2 1024 sur les vues de `sr3/longbowman.png` → planche
      `docs/img/ga3/s3_compare.jpg` (`tools/blender_scripts/ga3_compare_render.py`). Script fal hors
      dépôt `~/dev/cent-ans-raw/ga3/s3/fal_s3.py`, brutes au même endroit. Voir « S3 comparatif ».
- [x] S4 décor v2 : prompt corrigé (flux-2) → détourage bria → `trellis` et `trellis-2` sur la même
      image → `ga3_cleanup.py` étendu → `props_ga/ga3_house2_t{1,2}_lod*.glb` ; planche
      `docs/img/ga3/s4_house_v2.jpg` (locale, `docs/img/` ignoré sur main). Voir « S4 décor v2 ».

## S4 décor v2 (30/09)
Chaîne : `tools/experiments/ga3_fal_decor.py OUT --prompt-file P` (flux-2 1024² → bria → trellis /
trellis-2, chaque étape en cache, réponses fal gardées) puis `tools/blender_scripts/ga3_cleanup.py`
(nouvelles options `--exposure`, `--auto-levels P`, `--gamma`, `--normal auto|on|off`,
`--roughness auto|off` ; défauts = comportement S1, vérifié : 7 998 / 4 000 / 1 200 tri) ; planche
`tools/blender_scripts/ga3_sheet.py` (Cycles, même caméra/soleil) + `magick` pour l'assemblage.
Brutes (prompt, src, cut, glb, json) : `~/dev/cent-ans-raw/ga3/s4/`.
- Image : prompt « single small 14th-century French peasant cottage, one storey only with a low attic,
  … thatched roof, small smoke hole in the thatch and no chimney, … no brick, no glass … isolated on
  plain mid-grey, no ground, no grass » → un niveau, chaume, pans de bois, soubassement de moellons,
  pas de cheminée : anachronismes S1 réglés. Détourage bria propre → **plus aucun socle d'herbe**
  (`--strip-base` inutile).
- TRELLIS (0,02 $, ≈ 1 min) : 43,7 k tri bruts → 7 999 / 3 999 / 1 199, 7,99 × 5,93 × 5,62 m.
  Géométrie nette (pignon, débord du toit, soubassement), texture lisible.
- TRELLIS 2 1024 (0,30 $, même délai) : 98 k tri, albédo 2048 + metallicRoughness, **pas de normal
  map fournie** (le relief n'est que dans la géométrie). Décimé à 8 k, le chaume s'effondre : toit
  creusé, trous, faîtage déchiqueté (surface fine non manifold, 285 arêtes). LOD1/2 : la décimation
  « collapse » cale à ~5,7 k tri quoi qu'on demande → repli automatique par remaillage voxel
  (`decimated_copy`), 3 690 / 1 101 tri. Rugosité cuite depuis la source, normal map cuite.
- Albédo : `--auto-levels 1 --gamma 1.1` corrige S1 (terne) mais **sur-éclaircit** le chaume de la v2
  TRELLIS (paille presque blanche sous soleil) : pour le lot, `--auto-levels 0.5` sans gamma, ou
  exposition seule, à juger par objet. `--normal on` cuit le relief du maillage source sur les LOD
  (utile pour TRELLIS aussi).
- Écarts : pas d'import Godot lancé (pas de `.import` ni textures extraites pour `ga3_house2_*`, le
  lot d'intégration le fera ; éviter d'importer ici pendant que S2/S3 tournent) ; planche non commitée
  (`docs/img/` n'est plus suivi sur main, alors que S1/S2 le sont sur `feat/ga3` : à trancher à la fusion).
- **Verdict** : TRELLIS 2 ne vaut pas ×15 pour le décor de bataille : à budget de triangles égal (8 k),
  sa géométrie fine se décime plus mal et son PBR n'apporte presque rien vu de loin ; texture un peu
  plus juste en teinte, c'est tout. **TRELLIS + détourage** est la chaîne du lot.
- **Chaîne recommandée pour le lot décor** (maison ×2-3 variantes, église, moulin, chariot, tente,
  palissade, puits, trébuchet, bélier) : flux-2 1024² (0,012) → bria (0,018) → trellis 0,02 (seed
  fixe, 2 graines si raté) → `ga3_cleanup.py --normal on --auto-levels 0.5`. ≈ 0,05 $/essai,
  ~2 essais/objet → ≈ 1 $ pour ~10 objets. Objets fins (trébuchet, bélier, chariot : poutres, roues,
  cordages) : TRELLIS risque les mêmes fragments que l'arc de S2 → prévoir un essai `trellis/multi`
  (2-3 vues NB2) ou garder le procédural pour eux. TRELLIS 2 seulement pour un bâtiment-clé vu de
  près (église) et avec LOD0 relevé à 20-30 k : +0,30 $.
- [x] S5 végétation campagne (chêne + herbe/buisson + rocher) : voie A textures vs voie B TRELLIS →
      `tools/blender_scripts/ga3_vegetation.py`, candidats `game/assets/models/vegetation/ga3/` (rien de branché),
      planche `docs/img/ga3/s5_vegetation.jpg` (locale, `docs/img/` ignoré sur main). Verdict ci-dessous.

## S3 comparatif (30/09)
Entrée : les 3 découpes 848² de S2 (face, 3/4 droit, dos ; fond gris, non détourées). Un appel par
modèle, prix catalogue (l'API d'usage refuse la clé) : total **2,32 $**. Planche 1600 × 960, deux rangs
(3/4 face, 3/4 dos), même caméra/soleil EEVEE, chaque modèle mis à 1,80 m ; stats `s3/stats.json`.

| Modèle | Coût | Durée | Triangles | Textures | Arc | Fidélité |
|---|---|---|---|---|---|---|
| Tripo H3.1 multivue (tex. standard + géom. détaillée, PBR) | 0,50 $ | 166 s | **1 930 752** (sans `face_limit`) | 2048² albédo + ORM + **normal** | fusionné ; **deux arcs** (3/4 lu comme profil) | tenue, casque, carquois fidèles ; détail fin ; orientation −90° |
| Meshy 7.1 multi-image (tex. PBR, `a-pose`, rig 1,75 m, anim 224 Archery_Shot) | 1,52 $ | 419 s | 31 148 | modèle statique : 2048² albédo + métal + rugosité + **normal** ; rigué : albédo seul | **arc mis en bandoulière dans le dos** (A-pose), fusionné | propre, couleurs fidèles, mains en moufle, visage correct |
| TRELLIS 2 1024 (vue de face, `decimation_target` 30 000) | 0,30 $ | 58 s | 29 290 | 2048² albédo + métal + rugosité, pas de normal | tenu en main, fusionné | la plus fidèle de face ; dos lissé/plus pauvre |

Meshy : `model_glb` (statique PBR), `rigged_character_glb`, `animation_glb` (Archery_Shot, 120 images),
et **gratuits dans le prix** `basic_animations` marche (`walking_man`, 25 images) + course, en glb/fbx.
Défauts du glb rigué : métallique constant 1,0 et albédo branché aussi en émission (rendu chromé) — le
script les annule (`--unmetal`) ; les cartes PBR du modèle statique ne sont pas reportées sur le rigué
(même UV a priori, à recâbler). Squelette : **24 os** `Hips, Spine02 → Spine01 → Spine, neck, Head,
head_end, headfront, {Left,Right}{Shoulder, Arm, ForeArm, Hand, UpLeg, Leg, Foot, ToeBase}` — pas de
doigts. Le nôtre (`battle_fine.load_fine_human`, Quaternius ajusté par `battle_fine_rig.py`) : 62 os
dont 40 de doigts, `Root/Body`, pieds `Foot.*` enfants de `Root` à translations clés (IK) + pôles `PT.*`.
Correspondance : Hips→Hips, Spine02→Abdomen, Spine01→Torso, Spine→Chest, neck→Neck, Head→Head,
Shoulder/Arm/ForeArm/Hand→Shoulder/UpperArm/LowerArm/Wrist, UpLeg/Leg→UpperLeg/LowerLeg,
Foot→Foot (IK chez nous), ToeBase et doigts sans équivalent. **Retargeting faisable** (1:1 sur le
tronc et les membres) à deux conditions : cuire l'IK des jambes de nos clips en FK avant la copie, et
compenser la différence de pose de repos (A-pose Meshy / pose de liaison Quaternius) par rotations
en espace monde puis cuisson. Les doigts sont perdus (prise de l'arc et décoche en moufle).
Déformation de l'anim Meshy (vue sur 2 images) : bras levés propres, léger étirement de texture au
haut du torse/épaules, jupe du gambison qui suit les cuisses ; mais le tir se fait **mains vides**
(l'arc reste dans le dos).
Constat utile : la sortie `a-pose` de Meshy remplit d'office la condition de go de S2 (« A-pose mains
vides ») ; son maillage statique peut aussi être lié à *notre* squelette par la chaîne S2
(`fit_rig_to_mesh` + poids auto) sans retargeting, l'arc du jeu restant procédural.
Tripo : l'emplacement « gauche » refuse une chaîne vide (422) ; le 3/4 y a été placé tel quel, d'où
l'arc dédoublé. À refaire avec de vrais profils (face/gauche/dos/droite) si Tripo est retenu ; 1,9 M
triangles imposent `face_limit` (≈ 20-30 k) ou une décimation.

**Verdict prix/qualité pour 6 unités animées** : TRELLIS 2 (0,30 $) donne la meilleure fidélité mais
S2 a montré le coût du nettoyage (objets tenus) ; Meshy (1,52 $, ≈ 9 $ les 6) livre directement un
personnage propre en A-pose, PBR, rigué avec marche/course/1 anim, mais son squelette n'est pas le
nôtre. Rapport qualité-prix recommandé : **Meshy sans rig ni anim, texturé PBR en `a-pose`** (1,20 $,
7,20 $ les 6 — au-dessus du reliquat GA3) ou, moins cher, TRELLIS 2 sur une référence regénérée en
A-pose mains vides (0,30 $ + ≈ 0,08 $ d'image, ≈ 2,30 $ les 6), liés à notre squelette par la chaîne S2
pour garder nos clips et nos 40 os de doigts. Le rig/anim Meshy (+0,32 $) n'est utile que si l'on
abandonne nos clips Quaternius.

## Journal
- 30/09 : worktree créé, clé validée, joueur OK pour les 2 sondes.
- 30/09 : **S1 fait** (≈ 10 min de bout en bout, 0,05 $). Image `fal-ai/flux/dev` 1024² (0,026 $,
  6 s ; brutes `~/dev/cent-ans-raw/ga3/s1/`). Prompt : « Photorealistic photograph of a small
  14th-century French peasant half-timbered house, exposed dark oak timber framing with lime-washed
  wattle and daub infill, steep thatched roof, … Three-quarter view from slightly above, … isolated on a
  plain uniform neutral mid-grey studio background, no ground, … soft diffuse overcast lighting » →
  `fal-ai/trellis` (0,02 $, 25 s ; `texture_size` 1024, `mesh_simplify` 0,95, seed 1337) : 25 409 tri,
  UV + texture 1024² déjà fournis → `ga3_cleanup.py … ga3_house --strip-base 0.35` (Blender, < 1 min) :
  LOD0 7 998 / LOD1 4 000 / LOD2 1 200 tri, 8,00 × 6,77 × 7,71 m (h), pied à y = 0, albédo recuit
  1024/512/256 (Cycles, sélection vers active). glb 953/454/137 Ko. Contrôle `ga3_s1_shot.gd`
  (headless sans `--out`) OK. Planche `docs/img/ga3/s1_house.jpg`.
  Défauts : (1) TRELLIS reconstruit le socle d'herbe de l'image → `--strip-base` le retire, restent
  quelques touffes collées au pied des murs ; demander « no ground » ne suffit pas à flux, prévoir un
  détourage (fond retiré) avant TRELLIS pour le lot. (2) Faces cachées inventées (porte absente du
  côté arrière, pignon arrière plus pauvre) : acceptable vu de loin en bataille. (3) Albédo un peu
  sombre et terne (texture TRELLIS + ombres cuites de l'image) ; pas de normal map. (4) Anachronismes
  venus de l'image (cheminée de briques à mitre rouge, deux niveaux) : à régler au prompt.
  (5) Godot extrait les textures embarquées à côté des glb (`*_albedo.jpg`, commités avec les .import).
  **Avis : go pour le lot décor** (qualité très au-dessus du kit procédural pour ≈ 0,05 $/objet) avec
  deux ajouts au pipeline : détourage de l'image source et correction d'exposition de l'albédo ; les
  maisons de bataille sont vues de loin, le LOD0 8 k est confortable, LOD2 1,2 k suffit au-delà de 80 m.
  Point ouvert : `game/bin/` sans dylib dans ce worktree (sans effet sur la sonde, `core/build.sh` si besoin).
- 30/09 S2 (figurine) : `tools/blender_scripts/ga3_figure_probe.py`, planche `docs/img/ga3/s2_archer.jpg`,
  brutes `~/dev/cent-ans-raw/ga3/s2/` (vues découpées, `fal_trellis.py`, `multi.glb`, rendus, `.blend`).
  - Génération : `fal-ai/trellis/multi` (face + 3/4 + dos), 25 s, 0,02 $. Maillage brut 15,9 k tri, 1 seul
    îlot utile ; texture 2048² lisible (gambeson matelassé, chausses vertes, chapel de fer), mais albédo
    assombri (éclairage cuit), visage flou. Pas d'alternative fal essayée (résultat exploitable).
  - LOD0 : corps décimé à 11,1 k tri (plafond des fantassins fins 10,1-11,9 k) ; LOD1/LOD2 non faits.
  - Arc : inexploitable (branche supérieure fragmentée, membres de 1-2 cm perdus) → découpé et remplacé
    par l'arc procédural `battle_fine_weapons.longbow` sur `Wrist.L` (268 tri). Restes de branche
    collés à la main gauche et le long de la jambe (découpe géométrique imparfaite).
  - Rig : squelette `human` aux proportions fines, bras/jambes du squelette posés sur le maillage
    (écarts 34° bras gauche tenant l'arc, 15° bras droit, 12-14° jambes : le rig a une posture
    décalée), poids « heat » sur un rig de segments (les os Quaternius sont des moignons de 7 mm :
    la chaleur directe donnait les pieds à `Body`), puis ramené en pose de liaison ; clips
    `bow_walk` / `bow_shoot` inchangés. 100 % des sommets pondérés.
  - Déformations : marche correcte à distance de jeu ; épaules acceptables au tir ; le carquois
    que l'homme tient dans la vue 3/4 est fusionné à la main droite et monte au visage au tir ;
    pas de mains séparées (doigts fondus) → prise de l'arc approximative ; jupe du gambeson rigide
    entre les cuisses (étirement léger à la marche).
  - Variantes : texture cuite, aucune couleur de livrée ni blason (le pipeline actuel teinte par code
    de matériau + 2-3 variantes par figurine). Possible seulement avec un masque de recoloration
    (segmentation de l'albédo par teinte, à construire) ; blason = décalque à ajouter.
  - Coût par unité : ~0,02 $ (TRELLIS) + la planche NB2 de référence (~0,04-0,13 $) ; ~20 min de
    machine pour le script, mais retouche manuelle par unité (découpe arme/objets tenus).
  - Verdict S2 : **no-go en remplacement direct** des soldats de bataille ; **faisable techniquement**
    (triangles, rig, clips OK). Conditions d'un go : références en A-pose mains vides sans objet
    tenu (armes et carquois procéduraux), masque de livrée (zones teintables) + décalque de blason,
    albédo « dé-éclairé » (délighting) et intégration au format `CAM2` (atlas de détail, LOD1/2).
- 30/09 S4 (décor v2) : flux-2 + bria + trellis / trellis-2 (0,35 $), `ga3_cleanup.py` étendu
  (étalonnage albédo, cuisson normal/rugosité, repli voxel de la décimation), `ga3_sheet.py`,
  `ga3_fal_decor.py`. Verdict : TRELLIS + détourage pour le lot (~1 $), TRELLIS 2 non retenu.
- 30/09 **S5 végétation campagne** (0,21 $ ; brutes `~/dev/cent-ans-raw/ga3/s5/`, dont `fal_log.json` avec les
  prompts). Script `ga3_vegetation.py` en étapes : `fal` (flux-2 1024² → bria → trellis pour B), `atlas`
  (normalisation FC5 : couleur / moyenne linéaire × 0,5, moitié sapin conservée), `decimate` (Blender),
  `sheet` (cellules Cycles : même teinte de palette ×2 et découpe alpha 0,5 que `foliage.gdshaderinc`),
  `montage`. Lit l'arbre `_mid` et l'atlas FC5 dans le checkout `main` (FC n'est pas dans `feat/ga3`).
  - **A textures** : `ga3_leaf_cards.png` (1024 × 512, moitié gauche = amas de feuilles de chêne flux-2
    détouré, couverture 66 % contre 33 %), remplaçant direct de `campaign_leaf_cards.png`. Sur la même
    géométrie `_mid` : feuilles lobées lisibles, mais le gain est faible — ce sont les 100 cartes et leur
    répartition qui font la silhouette « chou ». `ga3_grass_tuft.png` : vraies couleurs paille/vert, mais
    brins fins → 13 % de couverture, la découpe à 0,5 en perd ; à régénérer en touffe plus dense.
  - **A' imposteur** (image flux-2 du chêne entier détourée sur un quad face caméra, 2 tri) : de loin le
    plus réaliste, proche comme en bosquet (silhouette, tronc noueux, trous de ciel). Limites : une seule vue
    (même arbre de tous côtés), éclairage cuit, pas de normal map, parallaxe nulle en plongée forte.
  - **B TRELLIS** : la décimation par effondrement cale à 3,3-4,8 k tri (centaines d'îlots de feuilles) ;
    remaillage voxel grossier (1/22) puis 250 / 90 tri (`ga3_oak_250/90.glb`, albédo 256/128) → **boule en
    pâte** confirmée, tronc avalé : no-go pour les arbres. Buisson 120 / 50 tri : boules texturées
    passables en couvre-sol lointain. **Rocher 120 / 60 / 18 tri** (`ga3_cleanup.py --lod0 120 --tex 256`,
    collapse direct) : bon, lichen lisible, go. Piège : matériaux TRELLIS métalliques → cuisson noire si
    `Metallic` n'est pas forcé à 0 (corrigé).
  - **Verdict** : voie **A'/A** pour la végétation (imposteurs générés + atlas de feuilles), **B seulement
    pour les rochers** (et au mieux les buissons lointains). Lot complet ≈ 1-1,5 $ avec reprises :
    3 essences × (1 carte de feuilles/rameau + 3-4 variantes d'imposteur) ≈ 15 × 0,03 $ ; 3 herbes + 3 buissons
    (cartes) ≈ 0,2 $ ; 4 rochers TRELLIS ≈ 0,2 $.
  - **Perf** : A = nul (même géométrie, même atlas 1024 × 512). A' = −248 tri par arbre proche si
    l'imposteur remplace les cartes, sinon nul ; texture : réutiliser la grille `campaign_impostors_albedo`
    (2048 × 768, 3 × 8 cases de 256²) → 0 draw call en plus. B rochers : 120 tri, 1 MultiMesh + 1 texture
    256² par type (ou un atlas commun → 1 draw call pour tous).
  - **Branchement** : A = copier `ga3_leaf_cards.png` sur `campaign_leaf_cards.png` (drop-in ; agrandir
    les cartes de 20-30 % si besoin). A' = générer la grille d'imposteurs (variantes dans les 8 colonnes, ou
    8 azimuts via nano-banana-2/edit), normal map plate ou dérivée de la luminance, et décider si les
    cartes `_mid` restent en deçà de 37-45 m. Rochers = nouveau genre dans `GroundClutter` (aujourd'hui
    herbe + buisson seulement) ou dans `Vegetation`, avec un MultiMesh dédié.
- 30/09 S3 (comparatif figurine) : 3 appels fal (Tripo 0,50 $, Meshy 1,52 $, TRELLIS 2 0,30 $ = 2,32 $),
  `fal_client.subscribe` bloqué après la fin des tâches Tripo/Meshy → résultats relus par
  `queue.fal.run/<app>/requests/<id>` (ids via `api.fal.ai/v1/models/requests/by-endpoint`). Planche
  `docs/img/ga3/s3_compare.jpg`, analyse dans « S3 comparatif ».

## Lots de production (go joueur 30/09)
- [ ] L1 décor : 10 objets flux-2 → bria → trellis → `ga3_cleanup.py --normal on --auto-levels 0.5`
      (maison paysanne, église [trellis-2, LOD0 20-30 k], moulin, tente, palissade, puits, chariot,
      trébuchet, bélier ; pièces fines : `trellis/multi` ou procédural gardé). Branchement décor de
      bataille (`battle_decor`) derrière option, ADR (prochain numéro libre). ≈ 1 $.
      **En cours (agent L1)** : catalogue `data/art/ga3_decor.json` (+ schéma, pytest),
      `ga3_fal_decor.py --catalog` (brutes `~/dev/cent-ans-raw/ga3/l1/`), ADR 0140 réservé.
      Suite : appels fal, nettoyage par objet, `Ga3Kit` (game/scripts/visual/ga3_kit.gd), test
      `ga3_l1_decor_test.gd`, planche `docs/img/ga3/l1_decor.jpg`.
- [ ] L2 végétation campagne : imposteurs générés (3 essences, 8 azimuts via nano-banana-2) dans la
      grille `campaign_impostors_albedo`, atlas de feuilles, herbe regénérée dense, rochers TRELLIS dans
      `GroundClutter`, derrière option. ≈ 1,50 $.
- [ ] L3 figurines : APRÈS fusion de `feat/sr`. Références A-pose mains vides → trellis-2 → chaîne S2
      (notre squelette, nos clips, armes procédurales), masque de livrée. ≈ 2,50 $.
- Verrou Godot partagé entre agents : `mkdir /tmp/ga3-godot.lock` avant `--import`/tests, `rmdir` après.
