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
Bevy n'expose pas le nombre d'appels de dessin, `visible_meshes` (entités `Mesh3d` visibles,
échantillon toutes les 10 images, lots instanciés comptés une fois) le remplace.
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
