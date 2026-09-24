# V6 — Performance du rendu (branche `visual-v6`)

Depuis `visual` (999ea1a, « V2b merged, main integrated »). Machine partagée (M4 Pro), mesures
bruitées (±30 %), chaque mesure répétée au moins deux fois.

## Objectif 1 — démarrage à chaud de la végétation

### Diagnostic
Profilage temporaire (chronomètres par étape, retiré avant commit) sur `vegetation_bench.gd` :
le semis des haies (`_scatter_hedges` dans `vegetation_tile_job.gd`) dominait très largement une
tuile (~850 ms sur ~1050-1150 ms), avec ~266 000 appels à `_hedge_point` par tuile. Deux causes :
1. `to_map`/`region_value` (déformation par sinus, 12-20 `sin()` par candidat) appelés deux fois
   par segment (point courant + point suivant), sans mise en cache.
2. La boîte englobante (u, v) des bords de haie, calculée sur les 4 coins du rectangle de la
   tuile, est très gonflée par le cisaillement des deux trames de parcelles (jusqu'à ~3× de
   surface en trop pour la trame la plus cisaillée, cisaillement `k = -0.8`) : la plupart des
   lignes/colonnes balayées ne croisent jamais la tuile et étaient rejetées tardivement par
   `rect.has_point`, après avoir payé le coût de `to_map`.
Piste explorée et abandonnée : sauter toute une trame de parcelles quand `region_value` ne
s'approche jamais du seuil d'acceptation sur la tuile — `region_value` oscillate en fait sur la
quasi-totalité de son intervalle dans presque chaque tuile (le terme `0,0131·x` parcourt plus
d'un demi-cycle sur 256 px), donc ce saut ne se déclenche jamais : abandonné (ne changeait rien,
ajoutait un flux aléatoire dédié par trame pour rien).

### Changements (`game/scripts/map/vegetation_tile_job.gd`)
- Grille grossière étendue : `region_value` (frontière des deux trames) précalculée sur la même
  grille que `crops`/`hedge`/`forest` (pas `coarse_step` = 4 px) et interpolée bilinéairement dans
  `_hedge_point`, au lieu de 4 `sin()` par candidat. Erreur d'interpolation négligeable (courbure
  de `region_value` très faible : ~0,005 sur 4 px, à comparer à la zone morte de 0,02 déjà dans le
  code).
- `to_map` mis en cache le long de chaque marche (bord à u constant, bord de rangée) : le point
  suivant d'un segment est le point courant du suivant, calculé une seule fois. Résultat
  bit-identique (mêmes valeurs de `v`/`u`, même flux aléatoire).
- Boîte englobante resserrée par ligne/colonne via une intersection linéaire exacte
  (`_clip_linear`) : `to_map` avant déformation est un plan affine de (u, v), donc la plage utile
  de `v` (ou de `row`) pour une ligne/colonne donnée se calcule sans trigonométrie. Les segments
  écartés étaient de toute façon rejetés par `rect.has_point` avant tout tirage aléatoire (premier
  test de `_hedge_point`) : aucun changement du flux de tirages pour les segments conservés,
  résultat bit-identique (nombre d'instances strictement inchangé après cette étape : 323 836).

### Changement (`game/scripts/map/vegetation.gd`)
- `warm_start_tiles` réduit de 12 à 5 (= `max_concurrent_jobs`). Mesure : `WorkerThreadPool` ne
  sert les tâches basse priorité (`add_task(..., false, ...)`) que sur ~4 fils quel que soit le
  nombre de cœurs disponibles (14 sur cette machine) — vérifié par la loi observée
  `warm_start_ms ≈ N_tuiles × temps_tuile / 4` pour N = 12, 8, 6. Demander plus que
  `max_concurrent_jobs` d'un coup n'ajoute que des salves d'attente bloquante supplémentaires sans
  plus de parallélisme réel. Testé aussi le sens inverse (passer les tâches en haute priorité
  pendant le démarrage à chaud) : nettement pire (~4,4 s), probablement parce que
  `wait_for_task_completion` exécute alors les tâches haute priorité directement sur le fil
  appelant plutôt que sur le pool — changement annulé.
  Effet visible : les tuiles au-delà des 5 les plus proches arrivent en quelques frames
  supplémentaires via le flux normal (déjà utilisé pour tout chargement en tâche de fond), au lieu
  d'être toutes prêtes à la toute première image. Vérifié par capture (`--stage=map --screenshot=`)
  et par le journal `Vegetation (warm start)` : rendu et densité inchangés, seul le délai d'arrivée
  des tuiles les plus excentrées de la salve initiale diffère (imperceptible en pratique).

### Mesures (`godot --path game --disable-vsync --script res://tests/vegetation_bench.gd`)
| | avant | après |
|---|---|---|
| `warm_start_ms` | 3365-3445 | 1145-1259 (1156-1164 en jeu réel via `--stage=map`) |
| `build_ms_max` | 1161-1197 | 852-898 |
| `instances` | 323 853 | 323 836 (−17 / 323 853, écart dû à l'interpolation de région, imperceptible) |

Cible < 1,5 s atteinte avec marge (mesures répétées 2-3 fois, cohérentes à ±10 %).

## Objectif 2 — sièges (MultiMesh)

### Changements
- `game/scripts/battle/battle_siege_batcher.gd` (nouveau, isolé comme demandé par l'orchestrateur
  pour la coordination avec la session qui branchera `battle_siege.gd` sur
  `BattleSim.get_siege().houses`) : `BattleSiegeBatcher.batch_and_replace(root)` parcourt un nœud
  déjà construit, regroupe les `MeshInstance3D` de type `BoxMesh`/`PrismMesh` par (matière, type de
  maillage) en `MultiMeshInstance3D` (maillage unitaire + mise à l'échelle par transform
  d'instance — `size` d'une `BoxMesh`/`PrismMesh` est un facteur d'échelle pur, la mise à l'échelle
  anisotrope via l'instance donne un résultat identique), et libère individuellement les nœuds
  récupérés. Ne connaît rien aux maisons/tours : prend n'importe quel nœud déjà construit par
  n'importe quelle logique de positionnement.
- `battle_siege.gd` : `_build_houses()` construit toujours ses 46 maisons exactement comme avant
  (position, tirage aléatoire, dimensions, choix de plâtre/toit/cheminée inchangés), mais sous un
  sous-groupe `Houses` passé en paramètre à `_house()` (nouveau premier argument), puis appelle
  `BattleSiegeBatcher.batch_and_replace(houses_root)` juste après. Les maisons sont statiques (pas
  de dégât suivi) : aucun risque de casser une mise à jour.
- Même traitement pour les tours (`_build_tower()`, sous-groupe `Towers`) : le fût conique
  (`CylinderMesh`, rayon/hauteur variables par tour) n'est pas regroupable par ce batcher générique
  et reste donc des nœuds normaux, mais les 3 archères par tour (`BoxMesh` de taille fixe) le sont
  — matière des archères mise en cache (`_slit_mat`, partagée entre tours, nécessaire pour que le
  regroupement par matière fonctionne). Le batcher gère nativement ce mélange (seuls les nœuds
  effectivement récupérés sont libérés, jamais leurs voisins non regroupables).
- Murailles (`_build_piece`) volontairement inchangées : dégâts par pan (brèche, effondrement,
  porte ouverte/fermée, teinte continue selon les PV) incompatibles avec un lot figé sans reprendre
  aussi la mise à jour par instance (hors budget de ce lot).

### Mesures (`godot --path game --disable-vsync res://scenes/battle/battle.tscn -- --benchmark --siege`)
| | appels de dessin |
|---|---|
| avant | 1164 |
| + maisons regroupées | 596 |
| + archères des tours regroupées | 556 |

FPS médian inchangé (60, plafonné par l'écran/le moteur dans cet environnement — 972 soldats, la
médiane n'était de toute façon pas limitée par le rendu du siège).

### Vérification visuelle
Capture `--siege --screenshot=` avant/après comparée par diff de pixels (maisons : diff moyenne
0,012/255 ; tours : 0,009/255, 232-275 px/1,3 M au-delà d'un seuil de 10 — bruit de rendu résiduel,
pas de tuile déplacée) et à l'œil (agrandissement des maisons et des tours) : aucun changement
visible (mêmes couleurs, mêmes colombages, mêmes toits, mêmes cheminées, mêmes archères).

### Points ouverts
- Fût des tours (corps, corbeau, parapet/toit) : ~40 appels de dessin restants, non regroupés
  (rayon/hauteur variables par tour, `CylinderMesh` non réductible à une simple échelle sans
  hypothèse par pièce sur le ratio rayon haut/bas — faisable mais pas fait faute de temps, gain
  marginal restant).

## Objectif 3 — zoom moyen de la carte (d≈300-500)
À faire (noté si non traité par manque de temps).

## Points ouverts
- `warm_start_tiles=5` fait dépendre le réglage du parallélisme réel du pool de threads bas
  niveau de Godot (mesuré ~4 sur cette machine) plutôt que d'une constante de conception : si une
  future version de Godot change ce comportement, revoir la valeur.
