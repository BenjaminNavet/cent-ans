# DN V1 : audit des éléments visuels de la bataille et des sièges

Lecture seule (08/10), aucune capture. Sources : `game/scripts/battle/`, `game/assets/models/`,
`data/art/`, `data/fx/`, `data/unit_types/`, `docs/pipeline-assets-3d.md`, bible DA § 6.
Légende rendu : **PROC** procédural GDScript/Blender script, **CAM1** figurine skinnée cuite
(`.mesh.bin`), **GA3** généré image → 3D (TRELLIS), **KIT** glb du kit Blender (`building_kit.py`),
**MM** MultiMesh, **IMP** imposteur.

## 1. Comment une figurine est branchée (à lire avant de remplacer une figurine)

Les figurines de bataille **ne sont pas des glb chargés tels quels**. Chemin réel :

1. **Format** : `<figure>_lod{0,1,2}.mesh.bin` (`CAM1` : sommets + indices + poids de peau + code de
   matière par face, zlib), + `manifest.json` et textures d'os (`human.bones.bin`, `cavalry.bones.bin`).
   Animations **cuites en texture d'os** (clips : marche, charge, mêlée, mort, etc.), jouées par le
   shader `game/shaders/battle_soldier_skinned.gdshader` en MultiMesh (un par régiment). Pas
   de `Skeleton3D`/`AnimationPlayer` à l'exécution. Chargeur : `game/scripts/battle/battle_skinned.gd`.
2. **Trois couches empilées** : figurines Quaternius `battle_skinned/` (repli `--coarse-figures`),
   figurines fines MakeHuman `battle_fine/` (**défaut**, ADR 0089, 9 infantry, 6 archer, 7 cavalry,
   crew, musician, standard, villager), puis **GA3** `battle_ga3/` qui *remplace* le maillage et l'albédo
   d'une figurine fine du même nom (`BattleSkinned._merge_ga3`). L'entrée fine garde style d'animation,
   noblesse, prises ; le manifeste GA3 apporte `lods`, `tris`, `variants`, `ga3_albedo`, `ga3_lum`
   (luminance moyenne livrée/chausses), `head_y`, `fine_horse`.
3. **Squelette** : rig partagé `human` (renommé `fine_human`) ou `cavalry` (`fine_cavalry`) ; le maillage
   TRELLIS est **auto-pondéré** sur le rig dans Blender (maillage ouvert → épaissi 12 mm, remaillé voxel
   8 mm, décimé au budget LOD0, UV Smart, albédo cuit Cycles). LOD1/2 = copies décimées (mêmes poids,
   mêmes UV, même texture). **Script de cuisson supprimé** par `a600b88f2` (SC) : récupérer
   `git show a600b88f2^:tools/blender_scripts/ga3_figures.py` (+ `ga3_fal_figure.py` dans
   `tools/experiments/`, qui contient les prompts de référence). Sortie :
   `blender -b --factory-startup --python tools/blender_scripts/ga3_figures.py -- <unit>`.
4. **Entrée image** : figurine **A-pose type « Christ de Rio »**, paumes ouvertes, **mains vides** (y
   compris cavalier), fond blanc ; **livrée en vert pur saturé**, **chausses en bleu pur saturé**, rien
   d'autre ni vert ni bleu. À la cuisson, ces zones deviennent des **niveaux de gris** (luminance) et
   l'**alpha de l'albédo = masque de teinte** (1 = livrée/chausses). Face `C_LIVERY` : le shader
   remplace le gris par la couleur du camp × (luminance / `ga3_lum.x`) ; face `C_CLOTH` : étoffe naturelle
   par soldat (palette `PLAIN`) × (luminance / `ga3_lum.y`) ; `C_PLATE` si le modèle dit « métal »
   (casque) ; `C_EXACT` ailleurs (couleur générée telle quelle). Teint de la tête recalé au-dessus de
   `head_y`. Usure SR2 (boue, crasse) ajoutée par le shader.
5. **Armes, écus, hampes : jamais dans le glb généré.** Elles sont **procédurales**, fusionnées dans le
   même `CAM1` par `battle_fine_weapons.py`/`battle_fine_gear.py` (FIGURES de `battle_skinned_figures.py`,
   items nommés), **skinnées sur les os** `Wrist.L/R`, `Nock`, `Arrow`, `Prop` (cavalier : `R:Prop`),
   avec UV négatives en u (le shader ne les texture pas) ; l'écu/pavois porte le code `C_ARMS` (UV
   d'armoiries). Remplacer une arme = nouvelle recette Blender, pas un glb à greffer.
6. **Cheval** : le cavalier généré monte le cheval fin exporté (`fine_horse: true`, source OGA CC0,
   harnais/caparaçon procéduraux gardant l'atlas FG3) ; seul le cavalier est généré.
7. **Têtes** : `variants` = têtes greffées (visages MakeHuman) sur le cou, 2 par figurine GA3.
8. **Budgets (bible § 6)** : fantassin LOD0 ≤ 12 000 tris (dessiné < 12 m seulement), LOD1 ≤ 1 350,
   LOD2 ≤ 260 (porte les ombres) ; monté ≤ 17 500 / 2 100 / 550. Distances : LOD0 12 m, LOD1 jusqu'à
   75 m, LOD2 au-delà, **imposteur** (atlas cuit à l'ouverture de la bataille, 8 angles × 4 jeux de clips,
   `battle_impostors.gd`) au-delà de 300 m. Textures : un albédo par figurine, partagé par les 3 LOD.
9. **Procédure d'un intégrateur** : générer la référence (Qwen-Image-Edit-2511 local, règles ci-dessus) →
   rembg → TRELLIS (HF) ou SF3D → `ga3_figures.py` → copier `.mesh.bin` + albédo dans
   `game/assets/models/battle_ga3/` → ajouter l'entrée au `manifest.json` (nom = figurine fine remplacée) →
   `godot --headless --path game --import` → `smoke.gd`. Le lien unité → figurine est le champ `figure`
   (`^(infantry|archer|cavalry)_N$`) de `data/unit_types/*.json`.

**Couverture actuelle** : GA3 sur archer_0/1/2/4, cavalry_0/3, infantry_0-8, standard_1. Restent en fines
procédurales : archer_3/5, cavalry_1/2/4/5/6, crew_0/1, musician_0/1, standard_0, villager_0-3.
Pas de champ `figure` pour 13 types d'unité (longbowmen, crossbowmen, genoese, men_at_arms_foot,
flemish_pikemen, urban_militia, knights, mounted_sergeants, mounted_archers, bombard, trebuchet,
mangonel, siege_tower) : repli variante 0 de la famille (voir § 4).

## 2. Tableau d'inventaire

| Élément | Rendu actuel | Entrée | Candidat glb | Brief (EN) |
|---|---|---|---|---|
| Fantassins (9 variantes) | GA3 + CAM1 skinné | `battle_skinned.gd`, `battle_ga3/` | déjà GA3 ; affiner | voir § 3 |
| Archers (0/1/2/4 GA3 ; 3/5 fines) | GA3 / CAM1 | idem | oui (3, 5) | liste A |
| Cavaliers (0/3 GA3 ; 1,2,4,5,6 fines) | GA3 / CAM1 + cheval fin | idem, `battle_cavalry_gaits.gd` | oui (5 cavaliers) | liste A |
| Chevaux, harnais, caparaçons | OGA CC0 ajusté + PROC Blender | `battle_fine_horse.py`, `battle_fine_cavalry.py` | oui (cheval) | A-pose horse, rig `cavalry` |
| Armes (épée, lance, pique, arc, arbalète) | PROC skinné sur os | `battle_fine_weapons.py` | oui, rigide, par os | liste B |
| Écus, pavois | PROC `C_ARMS` + blason | `battle_fine_gear.py` | oui (déjà bon) | B |
| Porte-étendard, musiciens | CAM1 `standard_0/1`, `musician_0/1` | `battle_standards.gd` | oui (standard_0, musiciens) | A |
| Drapeau/étendard (tissu) | PROC quad + `battle_standard_flag.gdshader` | `battle_standards.gd`, `data/fx/battle_standards.json` | non (tissu shader) | – |
| Servants d'engin | CAM1 `crew_0/1` | `siege_crew_fx.gd` | oui | A |
| Cadavres | MM du CAM1 basculé | `battle_soldiers.gd` | non (réutilise figurine) | – |
| Armes lâchées | PROC boîtes MM | `battle_dropped_arms.gd` | oui, petit glb rigide | B |
| Sang, membres tranchés | décalques + MM balistique | `battle_blood.gd`, `battle_gore.gd` | non (shader) | – |
| Flèches/carreaux en vol et fichés | MM shader | `battle_volleys.gd` | non | – |
| Pieux d'archers, pavois plantés | PROC | `battle_volleys.gd` (`stakes`) | oui | C |
| Imposteurs lointains | atlas cuit des figurines | `battle_impostors.gd` | non (dérivé) | – |
| Trébuchet, bélier | **GA3 découpé** sur noeuds procéduraux | `siege_engines_fx.gd`, `models/siege/ga3_*` | déjà | – |
| Mangonneau, bombarde, tour de siège | PROC Blender (360 à ~800 tris) | `siege_engines.py`, `models/siege/*.glb` | **oui** | D |
| Échelles d'escalade | PROC `_make_ladder` | `battle_siege.gd:876`, `siege_assault_fx.gd` | oui | D |
| Murailles, courtines, créneaux | PROC boîtes + PBR Poly Haven triplanaire | `battle_siege.gd` `_build_piece` | partiel (modules) | E |
| Tours rondes, porte, donjon | PROC | `_build_tower`, `_build_keep` | oui (modules) | E |
| Maisons de ville assiégée | KIT 46 maisons MM (`battle_siege_batcher.gd`) | `BuildingKit`, `town_kit/` | oui (GA3 house déjà pour champ) | F |
| Murs qui s'effondrent, éboulis | blocs RigidBody PROC | `wall_collapse_fx.gd` | oui (éboulis) | E |
| Incendies | flipbooks + particules | `siege_fire_fx.gd` | non | – |
| Maisons, église, moulin, puits, tentes, charrette | KIT + **GA3** (share 0,5 à 1) | `battle_decor.gd`, `visual/ga3_kit.gd`, `props_ga/` | déjà ; variantes à ajouter | G |
| Camps : tentes, pavillons, feux, chevaux au piquet | KIT + GA3 tent, MM | `battle_decor.gd` l.377-420 | oui (pavillon, cheval piquet) | G |
| Charrettes, chariots de bagages | KIT `wagon` + GA3 cart | idem | oui (chariot lourd) | G |
| Vigne, meules, tombes, murs de cimetière | KIT | `battle_decor.gd` | oui | G |
| Clôtures de plessis | PROC poteaux + panneau | `battle_village.gd:285` | oui (segment) | C |
| Palissades de camp retranché | **GA3** `palisade` (repli PROC) | `battle_village.gd:360`, `Ga3Kit` | déjà | – |
| Arbres (chêne, peuplier, fruitier, haie) | PROC ramifiés + cartes feuilles photo, 3 LOD | `battle_trees.gd`, `data/art/tree_species.*` | non (bien fait) ; conifères à vérifier | H |
| Buissons, haies | PROC `BattleMeshes.tree("bush")` | `battle_meshes.gd:877`, `battle_terrain.gd:1656` | oui | H |
| Rochers de bataille | PROC `BattleMeshes.rock()` (MM) | `battle_meshes.gd:995`, `battle_terrain.gd:1439` | **oui** (GA3 rock_a/b/c existent, non branchés ici) | H |
| Herbe | MM touffes + shader | `battle_vegetation.gd` | non | – |
| Ponts | KIT pierre/bois | `battle_bridges.gd` | oui (arche) | F |
| Toile de fond villes emblématiques | glb landmarks | `landmark_backdrop.gd` | hors périmètre | – |
| Oiseaux, fumée, poussière, éclaboussures | shaders/particules | `battle_birds.gd`, `battle_smoke.gd`, `battle_effects.gd` | non | – |

## 3. Constats utiles à la priorisation
- Fantassins et cavaliers lourds déjà GA3 (9+2). Les **cavaliers légers/moyens** (1,2,4,5,6) et les
  **archers 3/5** servent 12 types d'unité : ce sont eux qui font encore « figurine MakeHuman ».
- Armes et écus procéduraux : acceptable mais c'est ce qui trahit le plus le contraste avec les
  corps générés (silhouette des armes, fer des pointes).
- Engins de siège : seuls trébuchet et bélier sont GA3 ; **tour de siège, mangonneau, bombarde, échelles**
  restent boîtes/prismes (360 à 800 tris).
- Murailles/tours/porte de siège : PROC en matière triplanaire ; fonctionnent à distance, plats de près.
- Rochers et buissons de bataille : formes basiques ; les rochers GA3 (`vegetation/ga3/ga3_rock_{a,b,c}`)
  existent mais `BattleMeshes.rock()` reste utilisé en bataille (à vérifier avant d'intégrer).
- Les cadavres, gore, volées, herbe, fumées sont dérivés ou shaders : hors périmètre glb.

## 4. Liste priorisée à générer (35 assets)

Gabarits : **A** figurine A-pose « Christ de Rio », mains vides, livrée vert pur, chausses bleu pur,
fond blanc, LOD0 ≤ 12 000 (monté ≤ 17 500) ; **B** arme/écu seul, rigide, sur fond gris, ≤ 800 tris ;
**D/E/F/G** décor sur fond gris, trois LOD (3 000 / 1 500 / 450 tris pour accessoires ; 8 000 pour bâti).
Tous « circa 1337-1453, semi-realistic, weathered, no modern elements ».

### P1 : figurines à fort impact (12)
1. archer_3 `francs_archers/scots/yaya` : French/Scots archer, padded jack, kettle hat, short bow, A-pose, green jack, blue hose.
2. archer_5 `culveriners` : hand-gunner with open sallet, quilted coat, leather bandolier, A-pose, green coat, blue hose.
3. cavalry_1 `druzhina` : Rus' mounted noble, lamellar and mail, pointed helm, A-pose, green kaftan, blue hose.
4. cavalry_2 `mamluk, steppe archers` : Mamluk horse archer, lamellar over robe, turban helmet, empty hands, green robe, blue trousers.
5. cavalry_4 `routiers/pronoiars` : lightly armoured mounted man-at-arms, brigandine, open helm, empty hands.
6. cavalry_5 `jinetes/lithuanian` : light javelin rider, leather jerkin, felt cap, empty hands.
7. cavalry_6 `hobelars/akinci` : light mounted spearman on a small horse, gambeson, empty hands.
8. cavalry horse, bare (cheval) : medieval courser, saddle only, legs in neutral trot pose, side view, no rider.
9. cavalry horse barded : war horse with mail and cloth caparison in pure green, no rider.
10. standard_0 (porte-étendard à pied) : foot standard bearer, tabard green, arms raised wide, hands open.
11. musician_0/1 : drummer and horn-blower, A-pose, hands open, green tabard.
12. crew_0/1 : siege engineer in leather apron and cap, A-pose, hands open.

### P2 : engins et fortifications de siège (12)
13. mangonel (D) : 14th-century timber torsion mangonel with wooden frame, sling arm, ropes, 3 m long.
14. bombard (D) : large forged iron bombard on a heavy timber bed with iron hoops, wooden mantlet raised.
15. siege tower / belfry (D) : four-storey timber belfry on wheels clad in wet raw hides, drawbridge up, 12 m high.
16. scaling ladder (D) : long rough wooden scaling ladder with iron hooks, 8 m.
17. mantlet / wooden shield screen (D) : large wooden pavise mantlet on a stand, planks with iron bands.
18. curtain wall module (E) : 14th-century limestone curtain wall segment with crenellated parapet, wall-walk, ashlar blocks, 6 m long, tileable ends.
19. round tower (E) : 14th-century round limestone defence tower with machicolations and conical slate roof.
20. town gate (E) : stone gatehouse with portcullis, double oak doors with iron straps, drawbridge, flanking towers.
21. wall breach rubble (E) : heap of fallen limestone blocks and mortar rubble, wide, 5 m.
22. hoarding (E) : timber hoarding gallery projecting from a stone parapet, with arrow slits.
23. townhouse siege variant (F) : narrow 14th-century timber-framed town house, two storeys, jettied upper floor, 6 m wide.
24. stone arch bridge (F) : medieval two-arch stone bridge with cutwaters, 14 m long.

### P3 : décor de champ de bataille (11)
25. field rock cluster (H) : weathered limestone boulder group, moss patches, 2 m, three variants.
26. bush (H) : dense hawthorn bush, 1.5 m, no leaves for winter variant.
27. hedge segment (H) : bocage hawthorn hedge with a few saplings, 4 m long.
28. wattle fence segment (C) : woven hazel wattle fence section with posts, 2 m long.
29. sharpened stake cluster (C) : bundle of sharpened oak stakes planted at an angle, archers' defence, 1.5 m.
30. war pavilion (G) : large 14th-century lord's canvas pavilion with pole and guy ropes, red-and-white striped, 8 m.
31. field tent (G) : small A-frame soldier tent of undyed canvas, 3 m.
32. baggage wagon (G) : heavy four-wheel medieval baggage wagon with canvas cover and spoked wheels, 6 m.
33. campfire with cauldron (G) : stone ring fire with iron tripod and pot, logs, 1 m.
34. pavise stack and barrels (G) : stack of painted pavises, barrels and sacks, camp stores, 2 m.
35. gravestone / wayside cross (G) : weathered stone wayside cross and gravestones, 1.5 m.

### Armes (B), en lot après les figurines (hors numérotation)
longsword, arming sword, poleaxe, billhook, 4.5 m pike head, lance with vamplate, longbow, windlass crossbow,
heater shield, pavise, kite shield : une ligne chacune, par ex. « 14th-century arming sword, steel blade,
leather-wrapped grip, simple crossguard, weathered, semi-realistic ». Brancher en rigide sur `Wrist.L/R`
ou `Prop`, ce qui demande d'extraire le code d'attache de `battle_fine_weapons.py`.

## 5. Risques et points ouverts
- Script de cuisson GA3 supprimé (voir § 1.3) : restaurer avant tout travail de figurine.
- Procédé figurines : Qwen-Image-Edit local lent (25-35 min/image), TRELLIS gratuit limité à 1-3 générations
  par jour (quota ZeroGPU). Les figurines existantes viennent de Nano Banana 2 + fal (payant, ADR 0140) ;
  les nouvelles passent par le procédé gratuit de l'ADR 0210, donc cadence lente : viser P1 en priorité.
- Cohérence : les nouvelles figurines doivent suivre la même référence A-pose et les mêmes `head_y`.
- Les 13 types d'unité sans `figure` retombent sur la variante 0 : question de données (`data/unit_types`),
  pas d'asset, à trancher par le propriétaire du chantier.
- Non vérifié par exécution (lecture seule) : usage effectif des rochers GA3 en bataille ; arbres
  conifères ; tris réels des glb du kit `town_kit` (manifeste : 50 à 900 tris, très bas).
