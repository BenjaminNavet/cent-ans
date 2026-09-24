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
- **Dette T11 (`test_portraits.py`)** : fait, voir commit dédié
  (`tools/tests/test_portraits.py`). `test_dry_run_makes_no_network_call` dépendait de l'état
  réel de `game/assets/portraits` (le CLI `assets portraits --dry-run` appelle
  `portraits.plan(limit=limit)`, dont `out_dir` par défaut est ce dossier) : une fois tous les
  portraits générés, `plan()` renvoyait 0 job et le test cherchait « Style » dans une sortie
  vide. Le test reroute maintenant `portraits.plan` (monkeypatch) vers un `tmp_path` vide, donc
  toujours `limit` jobs, indépendamment du dépôt.
- **Dette T12 (RGBFloat)** : à faire (recherche de la source en cours).

## Mesures avant/après (T2)

Commande (fenêtre Metal, pas headless — le rendu a besoin d'un contexte GPU) :
`godot --path game --disable-vsync res://scenes/campaign_map.tscn -- --fps-probe
--focus=2213,1924,<d> [--fine-step=N]`, lib `libcent_ans.debug.dylib` (build fait dans ce
worktree, `core/build.sh`, après le lot T1 déjà en place côté orchestrateur).

**Machine très chargée pendant cette session** (plusieurs autres agents/`cargo build` en
parallèle dans d'autres worktrees, cf. audit § 2 sur le bruit habituel ±30 % même à vide) : les
i/s bruts ne sont pas comparables à ceux de l'audit (30 i/s ici contre 47-60 à vide). Le nombre
de primitives dessinées, lui, ne dépend que du pas choisi et reste comparable directement à
l'audit — c'est l'indicateur retenu pour confirmer le comportement adaptatif.

| Vue | primitives (audit, fine_step fixe) | primitives (ici, adaptatif) | i/s ici (bruité) |
|---|---|---|---|
| d=150 (comté, palier « près » large) | 8,8-9,4 M (`fine_step=1`) / 4,6 M (`--fine-step=2`) | **4,64 M** → confirme `fine_step_far=2` choisi automatiquement | 30,7 |
| d=45 (très proche) | 9,8 M (`fine_step=1`) | **9,79 M** → confirme `fine_step_near=1` choisi automatiquement | 29,1-32,0 |

Le choix adaptatif reproduit exactement les deux points de mesure de l'audit (4,6 M à d=150,
9,8 M à d=45) sans intervention manuelle : à d=150 (> seuil 90 avec hystérésis), le pas grossier
divise les primitives du relief fin par ~2 par rapport au pas fin par défaut d'avant ce lot,
pour le même gain que `--fine-step=2` mesuré dans l'audit (47-49 → 60 i/s plafond, machine à
vide) — et le rapprochement en dessous de d=70 repasse en pas fin sans régression visible
(maillages remplacés en place, jamais de retour au LOD proche intermédiaire, cf. § État).

## Commande du banc de bataille (T8)

```
godot --headless --path game res://scenes/battle/battle.tscn -- \
  --benchmark --units=120 --bench-at=40 --bench-timeout=180 --bench-repeat=3
```

Sortie : une ligne `BENCH_JSON {"ok":true,"units":...,"soldiers":...,"frames":...,
"repeats":...,"fps_avg":...,"frame_ms_median":...,"frame_ms_p95":...,"engine_fps":...,
"missiles_launched":...,"wall_s":...}` puis code de sortie 0 ; en cas d'échec,
`{"ok":false,"error":"...","wall_s":...}` et code de sortie 1.

## Vérification du banc de bataille (T8)

Fumée, machine partagée (chiffres non comparables à l'audit, seule la fiabilité compte ici) :

- `--units=20 --bench-at=5` : `BENCH_JSON {"ok":true,...}`, code de sortie 0.
- `--units=20 --bench-at=5 --bench-repeat=2` : 1200 images regroupées, `frame_ms_median`/`p95`
  cohérents, code de sortie 0.
- `--units=120 --bench-at=40 --bench-timeout=3` (budget volontairement trop court) :
  `BENCH_JSON {"ok":false,"error":"timeout advancing to --bench-at=40 (stopped at 29.9 s
  simulated)",...}`, code de sortie **1** — confirme que le banc échoue maintenant bruyamment
  au lieu de bloquer.
- **`--units=120 --bench-at=40 --bench-timeout=180`** (le scénario qui ne produisait jamais de
  résultat dans l'audit A5) : termine en 24,4 s de temps réel, `BENCH_JSON
  {"ok":true,"units":240,"soldiers":28752,"frames":600,"fps_avg":29.9,"frame_ms_median":31.0,
  "frame_ms_p95":55.2,...}`, code de sortie 0. Résolu (probablement la combinaison du profil
  `dev` optimisé du lot T1 et des garde-fous T8 ; l'ancien bug racine — `_ready()` ne quittait
  jamais le process en cas d'échec de mise en scène — est corrigé dans tous les cas).

## Prochaine étape

1. Fait : mesures avant/après T2 (primitives, voir ci-dessus).
2. Fait : banc de bataille à 120 régiments, succès et échec contrôlé (voir ci-dessus). Refaire
   à 48/80 régiments et à vide (machine non partagée) pour des i/s comparables à l'audit si
   besoin d'un chiffre de référence propre.
3. Fait : `tools/tests/test_portraits.py::test_dry_run_makes_no_network_call`.
4. Localiser la source des 35 avertissements RGBFloat→RGBAFloat au chargement de carte
   (recherche en cours).
