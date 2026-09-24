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
À faire.

## Objectif 3 — zoom moyen de la carte (d≈300-500)
À faire (noté si non traité par manque de temps).

## Points ouverts
- `warm_start_tiles=5` fait dépendre le réglage du parallélisme réel du pool de threads bas
  niveau de Godot (mesuré ~4 sur cette machine) plutôt que d'une constante de conception : si une
  future version de Godot change ce comportement, revoir la valeur.
