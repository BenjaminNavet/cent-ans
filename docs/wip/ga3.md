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
- [ ] S3 comparatif générateurs figurine (≤ 3 $) : Tripo H3.1 multivue / Meshy 7.1 multi-image
      (rig + anim) / TRELLIS 2 1024 sur les vues de `sr3/longbowman.png` → planche
      `docs/img/ga3/s3_compare.jpg` (`tools/blender_scripts/ga3_compare_render.py`). Script fal hors
      dépôt `~/dev/cent-ans-raw/ga3/s3/fal_s3.py`, brutes au même endroit. **En cours** : 3 appels lancés.
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

