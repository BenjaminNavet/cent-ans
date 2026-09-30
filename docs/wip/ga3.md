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

## L3a — figurine pilote longbowman (30/09)
Chaîne et choix : `docs/wip/ga3-l3.md` et ADR 0140 § « Figurines de bataille ». Brutes :
`~/dev/cent-ans-raw/ga3/l3/longbowman/`. Planche locale `docs/img/ga3/l3a_archer.jpg` (référence
A-pose | riggée liaison, marche, tir, dos à côté d'`archer_0` fin | marche des trois générateurs).
- Référence : `nano-banana-2/edit` 2K depuis `sr3/longbowman.png` : face et dos en A-pose
  propres, mains vides ; le profil tient le carquois (écarté). Jaque vert clé (livrée), chausses
  bleu clé (étoffe) → segmentation 35 % / 17 % de la texture.
- 3D : `trellis/multi` face + dos livré (consigne « moins cher ») ; `trellis` vue unique et
  `trellis-2` (parti avant la consigne) en comparaison. Les trois se valent une fois ramenés à
  11,5 k : multi garde un dos cohérent ; TRELLIS 2 n'apporte rien au format du jeu.
- Maillage : feuilles TRELLIS ouvertes et doublées (heat en échec, collapse calé) → solidify 12 mm
  + voxel 8 mm + collapse + cuisson. LOD 11 800 / 1 350 / 260 tri (arme comprise), heat 100 %.
- Jeu : rendu texturé en bataille (jaques de livrée rouge avec crasse, chapels, arcs, chausses
  variées), vu de dos et de trois-quarts ; `--no-ga3-fig` rend `archer_0` fin. Tests
  `ga3_l3_figures_test.gd` (deux modes), smoke, fg3_maps et sr2 (adaptés : `archer_0` n'a plus
  d'atlas), an1a, an1b, fk2, fk_folk, ga1_maps, nt7, nt10, nt12 OK.
- Écarts : capture Godot en bataille cadrée à côté de la troupe (caméra `--camera=572,595,13,205`,
  régiment décalé à gauche), jugement sur le bord de l'image ; une variante par figurine (plus de
  visages ni de couvre-chefs alternés) ; pas de FG3/SR2 sur la figurine générée.
- **Verdict** : go pour étendre aux 4 unités à pied (≈ 0,65 $) ; chevalier : générer le cavalier
  seul sur le cheval fin.

## L3b — quatre unités à pied (30/09)
Même chaîne que L3a, rendue générique **en données** (`UNITS` de `ga3_figures.py` : figure
remplacée, hauteur du casque, objets tenus par nom de la recette fine, `metal_z`, clips ; prompts
dans `ga3_fal_figure.py`). Planche locale `docs/img/ga3/l3b_units.jpg` (5 unités, marche et
attaque, GA3 | figurine fine actuelle, toutes deux dans les poses du jeu). 0,63 $.
- **Correspondances** (`data/unit_types` `figure`, sinon `BattleMeshes.VARIANTS`), une recette par
  type : homme d'armes `infantry_0` (hommes d'armes à pied ; épée + écu), arbalétrier `archer_2`
  (Génois ; arbalète, carquois, pavois au dos avec le drapeau BV3 `hide_pavise`), sergent
  `infantry_1` (piquiers flamands ; pique), milicien `infantry_5` (goedendag, Brabançons).
  Recettes du même type laissées fines : `infantry_7`/`8` (harnois, épée), `archer_1`/`4`
  (arbalète), `infantry_4` (pique), `infantry_2`/`3`/`6` (armes d'hast de milice).
- **Références** : 4 planches NB2 propres du premier coup (A-pose, mains vides, livrée verte
  unie sur jupon / tabard / surcot, chausses bleues ; l'homme d'armes n'a pas de clé d'étoffe :
  jambes de plates). Seul défaut : l'homme d'armes garde une épée au fourreau à la hanche
  (fusionnée au maillage, en plus de l'épée tenue).
- **3D** : `trellis/multi` face + dos, 1 génération par unité (≤ 3 permises, aucune reprise
  utile). Chaleur des os 100 % partout. Correctif : les jambes sous le tabard de l'arbalétrier
  sortent du voxel en îlots séparés (≈ 6 % chacune) : seuil de tri des îlots abaissé de 30 % à
  2 % (`KEEP_ISLAND`).
- **Équipement** : `equipment()` prend les objets par nom dans la recette fine (masque de
  variante, kwargs, surcharges SR3b) et les construit comme `build_figure` (registre fin
  `battle_fine_gear`/`battle_fine_weapons`, sinon V2). Les faces peintes (`C_ARMS` : écu, pavois)
  gardent leur UV d'armoiries et `GA3_TEX` les épargne (armoiries du jeu sur l'écu).
- **Acier** : `metal_z` par unité ; homme d'armes 0 : tout ce que la texture dit acier reste en
  `C_PLATE` ; le reste du harnois est en `C_EXACT` (couleur générée, jamais teinte).
- **Rendus** : le LOD0 exporté est skinné sur le CPU avec la texture d'os du jeu (comme la
  figurine actuelle) : l'arme suit les os virtuels (`Prop`) exactement comme en bataille (le
  rendu Blender posé plaçait l'arbalète et l'épée à l'horizontale).
- **Poids** : la chaleur des os fuyait des doigts (aliasés en `Wrist`) vers cuisses et tibias
  (AN1a : chausses du sergent déplacées) ; `strip_arm_leaks` retire les poids de bras sous
  0,6 m et renormalise ; faces de livrée sous 0,5 m rendues à la couleur générée. `archer_0`
  reconstruit avec ces correctifs.
- Tests : `ga3_l3_figures_test.gd` (5 figurines, témoins `archer_1`/`infantry_2`, deux modes),
  `an1a_motion_test.gd` (étendu aux figurines générées), `sr2_weathering_test.gd` (saute
  toutes les figurines générées), an1b, fg3_maps, nt7, nt10, nt12, fk2, fk_folk, smoke, pytest
  `tools/tests/test_ga3_figures_manifest.py` : OK.
- **Écart** : la seule capture Godot en bataille (gros plan de mêlée par défaut) cadre la
  cavalerie française : aucun fantassin généré visible ; jugement sur la planche (LOD0 exporté,
  poses du jeu). Capture ciblée (Crécy) laissée à la session principale.
- **Points ouverts** : variantes de visages (une figure par recette ; plusieurs générations
  par recette = variantes), mains en moufle, usure SR2 absente sur les figurines GA3, épée au
  fourreau fondue de l'homme d'armes, jupon sans armoiries, visages un peu rougis par
  l'étalonnage.

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
- 30/09 **L1 décor de bataille** (1,10 $ ; ADR 0140). 9 objets, tous réussis en TRELLIS vue unique
  (église en TRELLIS 2) ; triangles LOD0/1/2 : maison 7 999/3 999/1 199 (S4 recalibrée, aligné),
  église 24 997/12 499/3 749 (22 × 15 × 17,4 m), moulin sur pivot 7 999/3 999/1 199 (14 m),
  tente 3 000/1 500/450, palissade 2 998/1 500/446 (segment 3,1 m, pieux 1,9 m), puits
  2 999/1 499/1 100, charrette 3 000/1 500/847, trébuchet 2 999/1 499/641, bélier 3 000/1 500/824.
  `trellis/multi` (profil + dos regénérés par `flux-2/edit`) essayé sur les 4 pièces fines : pas
  mieux que la vue unique (débris en plus) → vue unique gardée. Trébuchet et bélier **non
  branchés** (engins animés `SiegeEnginesFx`). Branchement `Ga3Kit` derrière
  `BuildingKit.Batch.add` (maisons 50 %, église, moulin, puits 100 %, tente, charrette 50 %),
  palissade dans `BattleVillage`, tuiles LOD de 120 m ; `--no-ga3` ; rien sous la neige.
  Tests : `ga3_l1_decor_test.gd` OK (avec et sans `--no-ga3`), smoke OK, bataille headless
  « 16 GA3 variants » (0 avec `--no-ga3`). Capture Godot unique ratée pour le jugement :
  `--camera=` n'est pas appliqué avec `--open-shot` (vue par défaut, village trop loin) → jugement
  d'intégration en jeu à faire par la session principale (église vers (915, 707) sur la bataille
  par défaut). Défauts : enfoncement fixe 0,35 m (pas de fondations), toit du puits blanchi,
  peaux du bélier « vache pie ».
- 30/09 **L1b reprise décor** (0,17 $ ; brutes `~/dev/cent-ans-raw/ga3/l1b/`). Prompts réécrits dans
  `ga3_decor.json` (maison : chaume « very thick old straw thatch, dark grey-brown, smooth, evenly
  combed » ; puits : « dark weathered brown oak shingles » ; bélier : « thick dark oak planks covered
  with wet dark brown leather hides of one plain uniform colour », passé de `multi` à `trellis`).
  Nouveau : surcharge d'étalonnage par objet (`cleanup` dans l'entrée, `$defs/cleanup` du schéma,
  `exposure` ajouté ; fusion clé par clé dans `ga3_decor_build.py`). Maison : `auto_levels` 0,2 +
  `gamma` 1,15 (albédo moyen sRGB 0,18 ; 0,17 avec l'ancien 0,5 — le chaume sombre domine, le
  blanchi venait surtout de l'image S4). Un essai suffisait pour les trois. Triangles LOD0/1/2 :
  maison 7 998/4 000/1 200, puits 3 000/1 500/450, bélier 2 999/1 500/968. Planche
  `docs/img/ga3/l1_decor.jpg` refaite (3 × 3, 520 px) : chaume sombre et épais mais quelques
  facettes/creux TRELLIS au pignon droit, toit du puits en bardeaux brun-rouge sombre, peaux du
  bélier brun-rouge unies aux bords déchiquetés. Tests `ga3_l1_decor_test.gd` (72 / 0 avec
  `--no-ga3`) et smoke OK.
- 30/09 **L2 végétation campagne** (0,47 $ ; section « Végétation » de l'ADR 0140 ; note `docs/wip/ga3-l2.md`).
  `ga3_vegetation_l2.py` (brutes `~/dev/cent-ans-raw/ga3/l2/`) : par essence flux-2 → **une** planche
  nano-banana-2/edit 4 × 2 (8 azimuts cohérents) → bria ; grille `ga3/ga3_impostors_{albedo,normal}.png`
  au cadrage FC2 ; les imposteurs remplacent aussi les cartes proches (d = 25 : 2,40 M → 0,05 M
  triangles d'arbres ; Massif central, fenêtre : 17,0 M → 10,3 M primitives, ≈ 1 010 appels des deux
  côtés ; FPS non mesurables, machine chargée). Atlas de feuilles GA3 (si `--no-ga3-near`), herbe dense
  (41 %), rochers a/b/c 120/60/18 dans `GroundClutter` (enfant par cellule, `ground_rocks.gdshader`).
  `--no-ga3-veg` = état FC. Candidats S5 retirés de `game/assets/models/vegetation/ga3/`. Tests :
  `ga3_l2_vegetation_test.gd`, smoke, fc2 (forcé en FC), fc3, sz6 OK ; sz4b échoue pareil avec
  `--no-ga3-veg` (modèles de ville, préexistant). Planche locale `docs/img/ga3/l2_vegetation.jpg` :
  forêts lues en arbres distincts, plus claires ; rochers trop sombres corrigés après la capture
  (exposition), non revus en image.
- 30/09 S3 (comparatif figurine) : 3 appels fal (Tripo 0,50 $, Meshy 1,52 $, TRELLIS 2 0,30 $ = 2,32 $),
  `fal_client.subscribe` bloqué après la fin des tâches Tripo/Meshy → résultats relus par
  `queue.fal.run/<app>/requests/<id>` (ids via `api.fal.ai/v1/models/requests/by-endpoint`). Planche
  `docs/img/ga3/s3_compare.jpg`, analyse dans « S3 comparatif ».

## Lots de production (go joueur 30/09)
- [x] L1 décor (1,10 $, ADR 0140) : 9 objets flux-2 → bria → trellis → `ga3_cleanup.py --normal on
      --auto-levels 0.5` ; catalogue `data/art/ga3_decor.json` (+ schéma, pytest) ; brutes
      `~/dev/cent-ans-raw/ga3/l1/` et `l1p/` (pièces fines) ; `ga3_fal_decor.py --catalog`,
      `ga3_decor_build.py`, planche `docs/img/ga3/l1_decor.jpg` (`ga3_decor_sheet.py`). Voir journal.
- [x] L2 végétation campagne (0,47 $, ADR 0140 § végétation) : imposteurs générés (3 essences, 8 azimuts,
      une planche nano-banana-2 par essence) dans une grille au format `campaign_impostors_albedo`, aussi
      pour les arbres proches ; atlas de feuilles, herbe dense, rochers TRELLIS dans `GroundClutter` ;
      `--no-ga3-veg`, `--no-ga3-near`. Voir journal et `docs/wip/ga3-l2.md`.
- [x] L1b reprise de 3 objets L1 (0,17 $, un essai chacun) : maison (chaume sombre, lisse, épais), puits
      (toit en bardeaux de chêne sombres), bélier (planches + peaux brunes unies, vue unique). Voir journal.
- [x] L3a figurine pilote longbowman (`archer_0`, 0,48 $, ADR 0140 § figurines ; note
      `docs/wip/ga3-l3.md`) : planche NB2 A-pose à couleurs clés → `trellis/multi` face + dos →
      `ga3_figures.py` (voxel + cuisson, rig S2, LOD, livrée) → `battle_ga3/`, `GA3_TEX`,
      `--no-ga3-fig`. Voir « L3a ».
- [x] L3b : `man_at_arms` → `infantry_0`, `crossbowman` → `archer_2`, `sergeant` → `infantry_1`,
      `militia` → `infantry_5` (0,63 $, un essai chacune). Voir « L3b ».
- [ ] L3c : cavalier du chevalier sur le cheval fin (voir ADR 0140 § figurines, « Extension ») ;
      autres recettes des mêmes types (infantry_2/3/4/6/7/8, archer_1/4) si le joueur valide.
- Verrou Godot partagé entre agents : `mkdir /tmp/ga3-godot.lock` avant `--import`/tests, `rmdir` après.
