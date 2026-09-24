# T2 / T8 / dette — perf carte, banc de bataille, tests (agent T2)

Reprend l'audit `docs/audit/a5-technique.md` § 5, lots T2 et T8, plus une partie de la dette
(T11/T12 ciblée : `test_portraits.py`, avertissements RGBFloat).

## État

- **T2 (relief fin adaptatif)** : fait. `TerrainBuilder` (`game/scripts/map/terrain_builder.gd`)
  choisit `fine_step` selon la distance caméra au lieu d'une valeur fixe :
  - `fine_step_near = 1` en dessous de `fine_step_switch_distance` (80 par défaut),
    `fine_step_far = 2` au-dessus, avec une bande d'hystérésis (`fine_step_hysteresis = 20`,
    donc bascule à d=70 en approchant, d=90 en s'éloignant) pour ne pas reconstruire les tuiles
    en boucle au bord du seuil.
  - Reconstruction des tuiles affichées uniquement quand le pas a changé (`_fine_cache_step`
    par tuile), sans jamais repasser par le LOD proche (pas de « saut » visible) : la tuile
    garde son ancien maillage fin jusqu'à ce que la nouvelle version soit prête, puis
    remplacement direct.
  - `--fine-step=N` (CLI) garde son sens : force un pas fixe et désactive le choix automatique
    (`fine_step_auto = false`), utile pour les mesures avant/après ci-dessous.
  - Non traité (hors périmètre de la tâche confiée) : coupure des ombres du relief fin à
    d≈120 (suggérée dans l'audit T2 mais pas demandée explicitement) — laissé pour un lot
    suivant si le gain se confirme insuffisant.
- **T8 (banc de perf fiable)** : fait pour le banc de bataille
  (`game/scripts/battle/battle_scene.gd`, `--benchmark`) :
  - Sortie JSON systématique sur une ligne `BENCH_JSON {...}` (succès et échec).
  - Code de sortie ≠ 0 en cas d'échec : mise en scène impossible (`_stage_standalone`/`begin`
    échoués) ou budget de temps réel dépassé (`--bench-timeout=<s>`, défaut 120 s) — corrige le
    bug racine des « exécutions sans résultat » : `_ready()` ne faisait que `push_error` et
    laissait tourner la scène indéfiniment sans jamais appeler `get_tree().quit()`.
  - Vsync désactivé et `Engine.max_fps = 0` dès `--benchmark` (les i/s étaient plafonnées à 60
    sinon, cf. audit § 2).
  - `--bench-repeat=<n>` répète la fenêtre de mesure (600 images) ; tous les temps d'image
    (ms) sont regroupés pour calculer `frame_ms_median` et `frame_ms_p95` (pas seulement une
    moyenne d'i/s).
  - Garde-fou pendant `--bench-at=<s>` (avance rapide) : vérifie le budget de temps réel à
    chaque tick simulé au lieu d'une boucle bloquante sans limite (cause probable des blocages
    à 120 régiments).
  - Non reproduit : le « 8080 units » signalé une fois dans l'audit (aucune cause trouvée dans
    `_pad_setup`, appelé une seule fois par `begin()`) ; à surveiller, mais la sortie JSON
    systématique permet maintenant de détecter un résultat aberrant automatiquement.
- **Dette T11 (`test_portraits.py`)** : à faire.
- **Dette T12 (RGBFloat)** : à faire (recherche de la source).

## Mesures avant/après (T2)

Commande : `godot --headless --path game res://scenes/campaign_map.tscn -- --fps-probe
--focus=2213,1924,<d> [--fine-step=N]`, lib `libcent_ans.debug.dylib` (build fait dans ce
worktree, `core/build.sh`). Voir résultats dans la section suivante une fois exécutés.

## Commande du banc de bataille (T8)

```
godot --headless --path game res://scenes/battle/battle.tscn -- \
  --benchmark --units=120 --bench-at=40 --bench-timeout=180 --bench-repeat=3
```

Sortie : une ligne `BENCH_JSON {"ok":true,"units":...,"soldiers":...,"frames":...,
"repeats":...,"fps_avg":...,"frame_ms_median":...,"frame_ms_p95":...,"engine_fps":...,
"missiles_launched":...,"wall_s":...}` puis code de sortie 0 ; en cas d'échec,
`{"ok":false,"error":"...","wall_s":...}` et code de sortie 1.

## Prochaine étape

1. Lancer les mesures avant/après T2 (`--fps-probe` à d=150 et d=45) et les consigner ici.
2. Lancer le banc de bataille à 48/80/120 régiments avec les nouveaux garde-fous et consigner
   les résultats (avant, ces bancs ne terminaient pas de façon fiable).
3. Corriger `tools/tests/test_portraits.py::test_dry_run_makes_no_network_call` (répertoire
   temporaire au lieu de `game/assets/portraits`).
4. Localiser la source des 35 avertissements RGBFloat→RGBAFloat au chargement de carte.
