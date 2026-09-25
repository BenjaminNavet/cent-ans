# PB1 — Benchmark et optimisation des performances (FPS, chargements, zooms)

Demande : benchmarker le jeu et optimiser FPS + temps d'exécution (chargements, zooms) sans
dégrader le contenu. En pause à la demande de l'utilisateur, reprise prévue à 10h50 (2026-09-25).

## État
- Aucune modification de code encore.
- Bancs existants : `game/tests/v4_map_bench.gd` (carte), `game/tests/vegetation_bench.gd`,
  banc de bataille `--benchmark` dans `battle_scene.gd` (cf. `docs/wip/t2-perf.md`).
- Première mesure carte (debug dylib du 25/09 10:20, machine au repos) :
  - large d=1500 : 377 k prims, 1770 draw calls, 16,7 ms
  - Paris d=22 : 11,1 M prims, 760 DC, 16,6 ms
  - forêt d=90 : 7,5 M prims, 1278 DC, 16,7 ms
  - Loire d=40 : 11,9 M prims, 818 DC, 18,5 ms (54 i/s)
- **Problème du banc** : i/s plafonnées à 60 malgré `--disable-vsync --max-fps 0` et GPU = 0,00 ms
  → le banc ne mesure rien d'utile au-dessus de 60 i/s. À corriger en premier (vsync forcé par
  un script du jeu ? `viewport_set_measure_render_time` sur le mauvais viewport ?).

## Prochaines étapes
1. Réparer le banc carte (vsync/GPU), ajouter temps de chargement carte, temps de stabilisation
   après zoom (relief fin + végétation prêts).
2. Banc bataille : `--benchmark --units=20` et `--units=120 --bench-at=40`.
3. Chronométrer démarrage → menu → carte, chargement de bataille, tour de campagne (Rust).
4. Profiler, optimiser les points chauds, re-mesurer, consigner avant/après ici.
