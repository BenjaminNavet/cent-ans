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
