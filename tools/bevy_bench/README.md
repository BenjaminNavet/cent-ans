# Banc de la carte de campagne (Bevy)

Port de `game/scripts/dev/map_bench.gd` (voir `client/src/agent/bench.rs`).

    tools/bevy_bench/bench.sh [étiquette] [--bench-distance N] [--bench-seconds N]
                              [--bench-pan-only] [--bench-descent-only] [--offscreen]

Build release, fenêtre en arrière-plan (`client/run_bg.sh`), synchronisation verticale coupée.
Parcours : 90 images d'échauffement, panoramique Caen → Rouen → Paris → Chartres → Évreux à
d = 30 pendant 20 s, puis trois descentes (Rouen, Grande Chartreuse, Paris : 150 → plancher en
6 s, pause 1,5 s, remontée). Rapport JSON dans `out/bench/map_<étiquette>.json` (`pan` et
`descent` : `frame_ms_p50/p95/p99/max`, `fps_mean`, `worst_1pct_fps`, `frames_over_50ms`,
`visible_meshes_*`) ; code de sortie 1 si panique, `AGENT_FATAL`, rapport absent ou délai dépassé.

Écarts avec Godot : pas de phase « zoom » séparée, pas de statistiques du quadtree de relief ;
`visible_meshes` (entités `Mesh3d` visibles, échantillon toutes les 10 images) reste, les appels de dessin
sont dans `work` (voir plus bas).
Comparaison : chiffres Godot dans `docs/wip/fl-fluidite-carte.md`, `docs/wip/rx/perf.md`.

## Chronométrage par système (feature `profile`)

    BENCH_FEATURES=profile BENCH_TIMEOUT=1000 tools/bevy_bench/bench.sh <étiquette>

Le build est compilé avec `bevy_ecs/trace` (+ `debug` pour les noms) et une couche `tracing` maison
(`client/src/agent/profile.rs`, branchée via `LogPlugin::custom_layer` dans `main.rs`) : pas de
crate Chrome/Tracy (absentes du registre hors ligne ; `bevy/trace` échoue sur `profiling-procmacros`).
Le rapport gagne une section `systems` : `by_total` / `by_max` (système, appels, total, moyenne, max
en ms) et `spike_frames` (pour chaque image > 33 ms, les 5 plus longs systèmes qui s'y sont terminés).
Le traçage coûte cher (x2 à x3 en temps d'image sur un build chargé) : à lire en relatif, jamais
pour des chiffres d'image. `BENCH_TIMEOUT` couvre le build : le monter si le cache est froid.

## Banc de la bataille (`--bench-battle`)

    tools/bevy_bench/bench.sh <étiquette> --bench-battle [--bench-calm-seconds N] [--bench-melee-seconds N]

Ouvre Crécy (défenseur) par les menus (`battle_open.txt` : Batailles → Batailles historiques →
`Battle_crecy_defender`, prologue refusé) avec `CENT_ANS_NO_DEPLOY=1`, puis `client/src/agent/bench_battle.rs`
prend la main. Deux phases mesurées, mêmes champs que la carte (`frames`, `fps_mean`,
`frame_ms_p50/p95/p99/max`, `worst_1pct_fps`, `frames_over_50ms`, `visible_meshes_*`) :
- `calm` (15 s) : caméra d'ouverture figée (pivot, azimut, distance tels que posés par la scène), armées
  encore en approche ;
- `melee` (20 s) : simulation accélérée x16 jusqu'au premier régiment au corps à corps (sinon repli après
  3000 images, `contact: false`), puis vitesse x1, caméra oblique fixe (70 m, azimut d'ouverture + 35°)
  centrée sur le barycentre des régiments engagés à cet instant.
La caméra est réécrite à chaque image avant le contrôleur : le parcours ne dépend d'aucune entrée.
90 images d'échauffement ; synchronisation verticale coupée. Rapport : `out/bench/battle_<étiquette>.json`
(aussi `contact`, `sim_seconds_at_contact`, `regiments_at_end`, `entities_max`, `systems`).
Porte ADR 0404 : lire `melee.frame_ms_p95` ≤ 25 ms.

## Compteurs de travail et garde de performance (ADR 0827)

Le banc mesure le TRAVAIL par image, pas seulement le temps : un temps d'image dépend de la charge
de la machine (20-40 en permanence ici), un compteur non. Chaque phase du rapport (`pan`, `descent`
pour la carte ; `calm`, `melee` pour la bataille) porte un objet `work` :

| Champ | Sens |
|---|---|
| `draw_calls_p50/p95/max`, `triangles_p50`, `instances_p50` | appels de dessin soumis par image (tous les `TrackedRenderPass`, instanciés compris) ; triangles et instances des appels directs |
| `pipelines_created`, `pipelines_total` | pipelines ajoutés au `PipelineCache` pendant la phase ; total en fin de phase |
| `render_passes_p50`, `pass_cpu_ms` | passes de rendu par image et CPU moyen des plus coûteuses (`RenderDiagnosticsPlugin`, temps CPU seulement sous Metal) |
| `process_cpu_ms_*` | CPU de tous les fils du processus par image (`getrusage`). INDICATIF : les fils de travail tournent à vide plus longtemps quand la machine est saturée (+30 % mesuré), jamais un seuil |
| `main_thread_cpu_ms_*` | CPU du fil principal par image (`CLOCK_THREAD_CPUTIME_ID`, système exclusif donc toujours le même fil) : stable à ±10 % entre charge 5 et 40, c'est le seuil CPU |

Le bloc `memory` du rapport donne `footprint_mib`, `peak_rss_mib`, `metal_allocated_mib`, `heap_commit_mib`
(tas mimalloc) et `footprint_non_heap_mib`. Le dump d'état a la section `work` (240 dernières images ;
`CENT_ANS_COUNTERS=1` ajoute les passes) et `memory`. Avec `BENCH_FEATURES=profile`, chaque système
du rapport a aussi `cpu_total_ms` / `cpu_mean_ms` (CPU du fil, indépendant de la charge).

Les appels de dessin viennent de la feature cargo `perf-counters` (`bench.sh` la met toujours) :
`bevy_render/detailed_trace` fait émettre un événement `trace!` par appel, compté par une couche `tracing`
(`client/src/agent/counters.rs`) qui refuse tous les autres sites d'appel du module. Un build sans la
feature donne `draw_calls_*: null` et le garde échoue. Le comptage coûte un peu de CPU côté rendu : toutes
les valeurs de référence sont prises avec lui, les temps d'image d'un build sans lui sont un peu meilleurs.

    tools/bevy_bench/perf_guard.sh [map|battle|all]

Lance `bench.sh` (muet, `--offscreen`, aucun focus) et compare les compteurs de `perf_thresholds.json` (v2).
Code 0 = ok, 1 = seuil dépassé, compteur absent ou banc en échec. Plus de `max_load` ni de saut : les
compteurs tiennent sous charge (`CALM_SKIP=1` est posé par le garde). Les temps d'image sont affichés,
jamais comparés. `tools/full_check.sh` le lance (`PERF_GUARD=0` pour l'omettre).
Recalibrer après une fusion qui change le travail voulu : 3 exécutions, puis
`perf_calibrate.py tools/bevy_bench/perf_thresholds.json map out/bench/map_a.json out/bench/map_b.json out/bench/map_c.json`
(pire des trois + 5 % sur les comptes, + 20 % sur le CPU du fil principal ; médiane dans `reference`).
Réglage : `map_shadow_splits` (cascades du soleil de campagne) par préréglage dans `data/fx/render_quality.json` ;
`CENT_ANS_MAP_CASCADES=n` force le nombre pour un A/B (`ab.sh map <étiquette> "CENT_ANS_MAP_CASCADES=4" ""`).
Dans un worktree, lier les données ignorées (`data/map/pyramid`, `game/assets/models/dn`,
`data/map/compose/derived`, `game/assets/textures/*/hi`) vers le dépôt principal, sinon le banc mesure une carte sans relief.

## Comparaison A/B

    tools/bevy_bench/ab.sh <map|battle> <étiquette> "<envA>" "<envB>" [paires=3]

Compile une fois (release), exécute A B A B A B en alternance (`env <envA> bench.sh …`), puis affiche la
moyenne de p50 / p95 par variante (`pan` pour la carte, `calm` et `melee` pour la bataille) et le delta B−A.
Exemple : `ab.sh battle shadows "" "CENT_ANS_PERF_OFF=shadows"`. Sur machine chargée seuls les deltas
alternés ont un sens. Rapports : `out/bench/<mode>_ab_<étiquette>_{A,B}<i>.json`.
