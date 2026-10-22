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

## Garde de performance (lot B5)

    tools/bevy_bench/perf_guard.sh [map|battle|all] [--force]

Lance `bench.sh` (muet, `--offscreen`, aucun focus) et compare les percentiles aux seuils de
`perf_thresholds.json` (carte : panoramique d=30 p50/p95 ; bataille : Crécy calme/mêlée, mêlée p95 <= 25 ms,
ADR 0404). Code 0 = ok, 1 = seuil dépassé ou banc en échec, 3 = sautée car charge > `max_load` (6) ;
`--force` mesure quand même. Les seuils ne valent que sur machine calme.
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
