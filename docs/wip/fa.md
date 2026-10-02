# FA — assets libres pour la qualité visuelle (2026-10-02)

Mandat du joueur : « cherche des assets gratuits sur internet et utilise-les si pertinents pour
améliorer la qualité visuelle du jeu », autonomie pour télécharger et modifier ; ajout en cours de
route : « cherche aussi des animations ». Budget 0 $.

Branche `feat/fa`, worktree `../gp-fa`. Brutes hors dépôt : `~/dev/cent-ans-raw/fa/`.
Périmètre : batailles et effets. La carte de campagne appartient à la session TB (`feat/tb`) :
FA n'y touche pas.

## État
**Terminé et fusionné dans `main` le 2026-10-02.** Décision : ADR 0164. Voir « Restes ».

## État des lieux (ce qui reste procédural et qu'un asset libre bat)
- Arbres de bataille : rameaux `leaf_spray.png` dessinés (ovales), identiques pour toutes les
  essences → houppiers « en boules » de près (capture `da6_trees_shot`).
- Feu et fumée : planches `flame_flipbook.png` / `smoke_flipbook.png` synthétiques
  (`tools/cent_ans_tools/fire_flipbooks.py`).
- Animations : clips keyframés maison ; essai CMU (NT12) peu concluant
  (`docs/research/mocap-gratuite.md`).
- Déjà couvert par du CC0 (ne pas refaire) : sols, matières de bâtiments, ciels, figurines,
  polices, sons (voir `CREDITS.md`). Audit de départ : `docs/audit/a4-assets-libres.md`.

## Lots
- [x] FA1 — Feuillages des arbres de bataille : vraies feuilles photographiées (ambientCG
      LeafSet, CC0), un rameau par essence (`leaf_spray_<essence>.png`, 1024², mipmaps, BC7),
      `dead_leaves_oak.png`. Données `data/art/battle_tree_leaves.json`, script
      `game/assets/textures/battle/build_fa_leaf_sprays.py`, `BattleTrees.leaf_texture`,
      `--no-fa` pour l'A/B. Jugé sur captures : gain net de près (feuilles lisibles, houppiers
      plus fournis), neutre de loin. Piège : compression S3TC = reflets vert fluo sur les
      imposteurs → compression haute qualité.
- [x] FA2 — Feu et fumée : `tools/cent_ans_tools/vfx_flipbooks.py` recode **Flame03** et
      **WispySmoke03** (Unity Labs, CC0) au format des planches du lot V3 (aucun shader modifié),
      réglages `data/fx/fire_flipbooks.json`. Jugé en jeu (branche `feat/fa-fx`, 18 planches lues) :
      flammes nettement meilleures (brasier aux langues déchiquetées au lieu de trois larmes
      lisses), fumée meilleure (volutes irrégulières au lieu de boules), bombarde non dégradée.
      Flame02 (pied blanc carré, haut éteint) et Cloud02 (disque plein = boules opaques) écartés.
      Réglages ajoutés à l'outil : `heat_max`, `coverage_gamma` (flammes), `density_floor`,
      `density_gamma`, `edge_fade` (fumée). `density_floor` 0,04 retire le voile diffus que le
      gamma relevait en halo (alpha nul sur l'anneau à 90 % du rayon, test pytest).
      Bombarde : `game/tests/fa2_bombard_shot.gd` (âges de fumée fixes, graines fixées, gros
      plan et vue moyenne) ; planche `~/dev/cent-ans-raw/fa/fx-shots/fa2-bombarde-avant-apres.png`.
      Le grand disque pâle vu sur les captures de bombarde n'est pas la planche : c'est la
      traînée du boulet (`siege_assault_fx.gd`, `_trail_mat`, disque flou) qui s'empile à la
      bouche quand la simulation est en pause ; présent à l'identique avec l'ancienne planche. `data/fx/siege_fire.json` : flammes 3,5-7 m → 4,5-8,5 m.
      `s2_fire_shot.gd` réparé (caméra propre par-dessus les toits, interface écartée,
      `--flipbooks=<dossier>` pour l'A/B, aussi sur `sg3_siege_shot.gd` ; aide
      `game/tests/fx_flipbook_override.gd`). Planche avant/après :
      `~/dev/cent-ans-raw/fa/fx-shots/fa2-avant-apres.png`.
      **Non vérifié à l'image** : feu naval (`naval_fire.gd`), fumée de camp en campagne
      (`army_figures.gd`, périmètre TB) et feux de camp de bataille (source posée dans un siège
      invisible avec l'ancienne planche comme avec la nouvelle : mise en scène à revoir, pas un
      effet des planches). **Défaut restant** : fumée brun-rouge près des foyers (lueur et
      lumières ponctuelles du shader, antérieur à FA2) ; jeune fumée encore très sombre et dense
      juste au-dessus des flammes.
- [x] FA3 — Animations : reciblage Mesh2Motion + KayKit (CC0) sur le rig fin
      (`tools/blender_scripts/fa3_anim_retarget.py`, `data/fx/fa3_anim_sources.json`). Jugé sur
      planche Blender et en mêlée : `parry`, `death`, `death_back` par défaut (champ `default`) ;
      les dix autres clips, équivalents ou moins bons que les clips maison, derrière `--fa-anim`
      (`--no-fa-anim` = aucun). Note `docs/wip/fa3-anim.md`.
- [x] FA5 — Interface : sceaux de cire (Met Open Access 466080, 466082) et boutons cuir et laiton
      de l'accueil (ambientCG Leather030, Metal007), `game/scripts/ui/fa_ui.gd`,
      `data/ui/fa_ui_assets.json`. Lettrine et rinceau réels (Cleveland) écartés : flous à 50 px,
      moins bons que les dessinés. Note `docs/wip/fa5-ui.md`.
- [x] FA6 — Campagne, vue parchemin seulement : rose des vents et sirènes de l'Atlas catalan de
      1375 (`data/map/parchment_ornaments.json`, `use` par genre). Navires : restent dessinés (le
      navire de l'Atlas et la nef de Ferrer sont illisibles à l'échelle de la carte).
      `--no-fa-parchment` pour l'A/B. Note `docs/wip/fa6-parchemin.md`.
- [x] FA7 — Herbe des batailles : atlas de douze touffes de vrais brins (ambientCG Foliage, CC0),
      `data/art/battle_grass.json`, `--no-fa-grass` pour l'A/B. Gain le plus net du chantier,
      5 à 9 % moins cher à l'image. Note `docs/wip/fa7-herbe.md`.
- [x] FA4 — ADR 0164, captures avant/après (`~/dev/cent-ans-raw/fa/*-shots/`), fusion `--ff-only`
      dans `main`.

## Restes
- FA2 : feu naval et fumée de camp en campagne non vérifiés à l'image ; fumée brun-rouge près
  des foyers (antérieur à FA2).
- FA5 : sceau du traité signé non vu sur capture ; débordement du panneau des techniques en
  1280×720 (antérieur à FA).
- FA7 : semis moins lisibles de loin, plaques de sous-bois en forêt, fleurs presque invisibles,
  sang sur l'herbe non jugé, blé un peu uniforme.
- Rochers photogrammétrés Poly Haven (rock_07/09, rock_face_01, mountainside) : à proposer aux
  sessions de la carte 3D (TB, HC) pour `rock_outcrops.yaml`.
- Lacunes sans source libre téléchargeable sans compte : cogue médiévale, feuillus européens 3D
  (pistes itch.io par navigateur : Polyy.AI, Quaternius Ships), parchemin et nuages tuilables.
- Tests en échec avant FA et sans rapport : pytest `test_entity_icons`, `test_ink_icons`,
  `test_relief_update`, `test_water_detail` ; Godot `nv2_naval_test`, `po_ui_test` C3,
  `q6_ui_test`, `fe_ui_test`.

## Sources retenues (licence vérifiée)
- ambientCG (Lennart Demes), CC0 1.0 : `https://ambientcg.com/get?file=<id>_2K-JPG.zip`.
- Unity Labs Paris, flipbooks VFX, CC0 (billet
  `https://blog.unity.com/technology/free-vfx-image-sequences-flipbooks`, pas de fichier de
  licence dans les zips) : `https://unity3d.com/files/labs/downloads/vfx/assets01/<Nom>/<Nom>-flipbooks.zip`.
- Mesh2Motion (CC0, dépôt officiel GitHub) et KayKit Character Animations 1.1 (Kay Lousberg,
  CC0 ; récupéré d'un miroir GitHub tiers, licence citée depuis kaylousberg.com).
- Kenney Particle Pack / Smoke Particles, CC0 (sprites stylisés : secours seulement).

## Captures
Plafond levé par le joueur le 10-02 (« budget illimité de captures ») pour la session principale
et les agents visuels du chantier. Rester à ≤ 1280 px de large et assembler avant/après.

## Journal
- 10-02 : worktree, état des lieux, recherche effets terminée, recherche animations lancée.
- 10-02 : FA1 commité ; FA2 cuit, agent visuel lancé ; FA3, FA5, FA6 lancés en parallèle (4
  agents FA + 4 agents TB = 8 sur 10). pytest sur `feat/fa` : 1476 OK, 4 échecs hors FA
  (`test_water_detail`, `test_entity_icons` aussi sur main ; `test_ink_icons`,
  `test_relief_update` propres au worktree).
- 10-02 : FA1 vérifié en bataille (forêt, vue rapprochée et déploiement, avec et sans
  `--no-fa`) : lisières identiques de loin, aucune régression. FA7 lancé (5 agents FA + 4 TB).
- 10-02 : FA2 jugé et réglé en jeu (Flame03 + WispySmoke03), `feat/fa-fx`.
- 10-02 : FA2 fusionné dans `feat/fa` (670464d8d). FA6 : rose et sirènes validées, navire réel
  refusé (agent relancé). FA3 : chaîne validée, agent relancé pour passer `parry`, `death`,
  `death_back` par défaut via les données. Autres sessions sur la carte : TB (carte 3D), GC
  (champs, hameaux, moulins), HC (arbres, forêts, lacs, étangs ; ADR 0161 pris) — FA ne touche en
  campagne que la vue parchemin. Machine saturée (48 processus Godot, charge > 190) : ne plus
  lancer d'agent tant que les lots en cours n'ont pas rendu. Numéro d'ADR de FA à prendre au
  moment de la fusion (`ls docs/decisions | tail` dans main, 0153-0156 et 0161 réservés).
- 10-02 : **PAUSE demandée par le joueur.** Dans `feat/fa` (604d4f7ce) : FA1, FA2, FA5 (sceaux
  et boutons seulement), FA6 (rose et sirènes ; navires dessinés, nef de Ferrer retirée), FA7,
  ADR 0164. Agent FA3 arrêté en cours de route (`../gp-fa-anim`, `feat/fa-anim`) : il passait
  `parry`, `death`, `death_back` par défaut via un champ `default` de
  `data/fx/fa3_anim_sources.json` (`--fa-anim` = les 13, `--no-fa-anim` = aucun) et vérifiait en
  mêlée ; voir `git status` et `docs/wip/fa3-anim.md` dans ce worktree avant de reprendre.
  **Reprise** : 1) finir FA3 puis le fusionner ici ; 2) `godot --headless --path game --import`
  puis `smoke`, `cm2_parchment_weather_test`, `dv_two_views_test`, `fa5_ui_test`,
  `fa7_grass_test`, `s2_fire_fx_test`, `fa3_anim_test` et pytest complet sur `feat/fa` (la
  passe Godot post-fusion a été interrompue, non faite) ; 3) revérifier le numéro d'ADR, fusion
  de `main` dans un worktree puis `--ff-only` ; 4) supprimer les worktrees `gp-fa*`.
- 10-02 : reprise. FA3 jugé (parade et deux morts par défaut) et fusionné ; `main` fusionné dans
  `feat/fa` sans conflit ; passe de tests verte (smoke, `cm2_parchment_weather_test`,
  `dv_two_views_test`, `dv_markers_test`, `fa5_ui_test`, `fa7_grass_test`, `s2_fire_fx_test`,
  `fa3_anim_test`, `nt14_melee_test`, `ui1_lettrine_test`, `vn_ui_720_test`, `bv3_check`,
  `zg8_relief_test`, 37 tests pytest FA) ; fusion `--ff-only` dans `main`, worktrees `gp-fa*`
  supprimés.
