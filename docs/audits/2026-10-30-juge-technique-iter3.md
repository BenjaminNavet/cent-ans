# Juge technique, itération 3 de la boucle juge 2 (30/10/2026)

Auteur : juge technique frais (agent), sur `fusion-a` (c3db29c39). `fusion-a` n'est pas descendant de `main` : `main` porte 2 commits
absents (554d924e9 « warm worktree, heavy.sh timing log », ad133ad0c) ; la fusion devra les inclure (voir T-8).
Méthode : lecture ciblée, scripts de comptage, rapports de banc déjà produits le 29/10 (`out/bench/*juge2*.json`), notes des
branches MEM3 (`agent-a0377ad7d1eea36f9`) et FPS mêlée (`agent-a7fa6dbbe9cdf1f9d`). Aucun client lancé (charge 22-42 pendant
l'audit, 3 emplacements `heavy.sh` pris en permanence) : **aucun fps absolu ici**, uniquement du travail par image, des compteurs,
des tailles et des durées de tâches. Rien du code de jeu n'a été modifié.

## Constats

### Majeurs

**T-1. Les gardes de performance ne protègent rien : les seuils sont dépassés de 50-60 % et le garde ne tourne jamais.**
- `tools/bevy_bench/perf_thresholds.json` : empreinte `map` 7 800 Mo, `battle` 4 500 Mo, `max_load` 6. Le dernier banc complet
  (`out/bench/map_juge2full.json`, 29/10 16:30) donne **12 539 Mo d'empreinte** (pic RSS 9 101) en fin de banc carte ; la bataille
  (`battle_juge2battle.json`) **6 657 Mo** d'empreinte. Soit +61 % et +48 % sur les seuils « de non-régression ».
- `perf_guard.sh` saute (code 3) quand la charge dépasse 6 ; la charge de la machine est de 20-42 en permanence. Le garde n'a donc
  pas pu échouer. Les seuils en ms (pan p50 20, mêlée p50 17,5 / p95 25) sont des mesures de temps réel : inutilisables sur machine
  chargée, ce qui est le cas normal de ce dépôt.
- Conséquence : toute régression visuelle (VIS-R1/R2/R4 ajoutent du GPU « à mesurer plus tard ») passe sans signal. Voir lot **L-A**.

**T-2. Mémoire de campagne : 5,4 Go au repos, 12,5 Go en fin de banc, cible 3 Go « hors Metal » non atteinte ; les deux lots de
réduction sont des outils de mesure, pas des réductions.**
- `docs/performance-memoire.md` (ADR 0794/0795) : repos 5 400-5 490 Mo (tas CPU 2 340-2 390, GPU non mappé 2,7 Go), 7 350 en fin de
  banc (MEM4) ; le banc du 29/10 mesure 11-12,5 Go (`map_holes3` 11 053, `map_juge2map` 11 187, `map_juge2full` 12 539). Croissance
  de **~7 Go pendant les trois descentes** non attribuée : `MAX_PAGES = 256` (`client/src/campaign/terrain/lod.rs:26`) borne les
  pages de relief mais pas les caches de sol (`terrain/ground_cache.rs`), de détail, de végétation ni les tas de décodage.
- Branche MEM3 (`worktree-agent-a0377ad7d1eea36f9`, 7 fichiers, +314/-8) : dump `memory` élargi, journal de pipelines, scripts de trace
  Metal. `docs/wip/mem3.md` : « Mesures de réduction (KTX2, caches, ADR 0786) toujours à faire ». Rien ne réduit la mémoire. Les
  gains certains déjà identifiés restent non faits : ~100 Mo de copies CPU évitables (`performance-memoire.md` point 4), feuilles de
  sol JPEG décodées 185 Mo RGBA8 chacune (pré-cuisson KTX2 « non faite »), atlas d'imposteurs en copie CPU 330 Mo, rasters du
  semis 590 Mo (MEM5).
- Voir lot **L-B**.

**T-3. Une simulation complète de fin de tour s'exécute sur le fil principal, à chaque changement d'état, au plus d'une fois par seconde.**
- `client/src/campaign/hud/mod.rs:71` (`FORECAST_SECONDS = 1.0`) et `:376-381` : `game_api::forecast(...)` est appelé dans un système
  `Update`. `core/crates/game-api/src/hud.rs:339` → `sim_campaign::forecast::faction_forecast` (`forecast.rs:204`) →
  `simulate_next_turn` : copie de l'état + fin de tour complète (la note `docs/wip/rg8.md:17` donnait ~150 ms en release en septembre ;
  l'économie, les révoltes graduées, les États, la semonce ont depuis grossi la fin de tour). `season_story_with` rejoue ensuite
  des calculs sur le même état, sur le même fil.
- Effet : un pic d'image de plusieurs images à chaque clic d'ordre ou fin de tour ; pire en campagne qu'en bataille. Le banc
  carte ne le voit pas (pas de changement d'état pendant le panoramique), donc aucun seuil ne le couvre.
- **Mesure demandée, lancée mais non terminée** : `ai --example turn_perf -- --sequential 20 1 1` (temps CPU du fil, insensible à la
  charge) est resté en file `heavy.sh` plusieurs minutes faute d'emplacement (voir T-8). Résultat à ajouter au lot au moment de le lancer.
- Voir lot **L-C**.

**T-4. Le profil `release` du client est celui par défaut de cargo ; la simulation du jeu joué tourne sans LTO ni `codegen-units=1`.**
- `client/Cargo.toml` : aucune section `[profile.release]` (seulement `[profile.dev]`, lignes 105-112). Le client est un workspace
  séparé : le `[profile.release]` de `core/Cargo.toml:19` (thin LTO, `codegen-units = 1`, ADR 0079) ne s'applique **pas** aux crates
  `sim-campaign`, `sim-battle`, `ai`, `data-model` quand ils sont compilés par le client. Le binaire joueur et tous les bancs
  (`bench.sh` est « build release ») les compilent avec 16 unités de codegen, sans LTO, y compris la fin de tour de T-3 et la
  simulation de mêlée (65 911 entités max).
- Gain plausible à mesurer (pas de chiffre ici) ; coût : temps de compilation release plus long (ADR 0079 : thin LTO ≈ 10 min,
  fat LTO 32 min). Le profil `seed-bots` (core) montre que l'équipe sait faire l'arbitrage. Voir lot **L-D**.

### Moyens

**T-5. Mêlée à ~59 fps : liée au GPU, 4 900 `Mesh3d` visibles identiques en calme et en mêlée, pas de compteur de draw calls.**
- `battle_juge2battle.json` : mêlée p50 16,9 ms (59 fps), p95 18,0, max 40,3, `worst_1pct_fps` 43,6 ; calme p50 12,5. `visible_meshes_p50`
  4 889 (mêlée) contre 4 903 (calme) : le dénombrement ne bouge pas avec l'action, il ne dit donc rien du coût réel ; Bevy n'expose pas
  les draw calls (`tools/bevy_bench/README.md`). Le bench ne sait pas dire « combien de passes, combien de draws, combien de pipelines ».
- Branche FPS mêlée (`worktree-agent-a7fa6dbbe9cdf1f9d`, `docs/wip/lot-fps.md`) : préchauffage `flow/pipeline_warm.rs` (159 lignes,
  « NON vérifié par clippy/tests », désactivé par défaut depuis la reprise), 13 → 11 pipelines tardifs en bataille (une mesure),
  5 pipelines tardifs en carte. Sur macOS `create_pipeline_task` est synchrone (cfg macos) : pics de 350-1 100 ms au premier passage
  de chaque variante en carte (ADR 0794). Seule paire d'A/B « calme » : volumétrique 64 → 40 pas : −2,7 ms de mêlée. Les réglages
  `VOL_STEPS`, `SHADOW_DIST` ne sont que des variables d'environnement.
- Voir lot **L-A** (compteurs) et **L-E** (réglages et préchauffage).

**T-6. Chargement : bataille 2,3-2,5 s, `GameData::load` séquentiel, aucune cible de durée.**
- `startup_total_ms` : bataille 2 312 (`battle_juge2battle`) / 2 548 (`battle_bf1`) ; carte 454-961 selon le rapport. Pas de seuil dans
  `perf_thresholds.json`. `core/crates/data-model/src/load.rs` (1 998 lignes) charge ~2 400 fichiers JSON sans parallélisme
  (aucun `rayon` dans `data-model`) ; le décodage des feuilles de sol de campagne (185 Mo RGBA8 chacune) produit les pics de
  chargement de `performance-memoire.md`.
- À mesurer avant de décider : `GameData::load` isolé (temps CPU), part du décodage de sol, part des pipelines (T-5).

**T-7. Dette d'architecture : un crate client de 251 584 lignes, des règles et des textes dans le client, 1 051 constantes numériques de
rendu codées en dur.**
- `client/src` = 251 584 lignes en un seul crate (`cent_ans_client`, 1 543 `#[test]`) contre `sim-campaign` 73 994, `sim-battle` 37 594,
  `game-api` 14 502 : tout changement de client recompile et relie le crate entier ; les tests du client sont un seul binaire. Les
  domaines (`campaign`, `battle`, `menu`, `codex`, `audio`) se prêtent à un découpage en crates (`client-ui-kit`, `client-battle`,
  `client-campaign`, `client-core`) avec features dynamiques conservées.
- Règles hors `core/` (détectées par lecture, petites mais contraires à l'invariant) : `client/src/menu/custom_battle.rs:277`
  `army_cost` (somme de `u.cost.money`, validation du coût de bataille perso) ; `client/src/campaign/panels/army/basket.rs:41-48`
  `total_cost`/`total_upkeep` et `can_seal` (:116) recalculent budget et entretien du panier de recrutement à partir de `RecruitView`.
  Pas de fuite grave, mais deux sources de vérité pour le coût.
- Constantes : 1 051 `const X: f32/u32/... =` dans `client/src` (33 dans `battle/units/skinned.rs`, 32 dans `campaign/towns/far/builder.rs`,
  24 dans `battle/scene/camera.rs`, 20 dans `battle/units/volleys.rs`) contre 240 dans `core/`. La règle « jamais de données codées en dur »
  vise surtout les règles ; pour le rendu elle n'a pas été appliquée, ce qui rend le réglage visuel/fps impossible sans recompiler
  (le lot FPS a dû passer par des variables d'environnement, T-5).
- Textes : ~777 chaînes françaises dans le code de `core/` (`sim-campaign/src/orders/error.rs` 30, `invariants.rs` 27, `capture.rs` 26,
  `ransom.rs` 24, `negotiation/value.rs` 22…) et ~746 dans `client/src` (`settings/rows.rs` 57, `settings/keymap.rs` 49,
  `court/view.rs` 39…), alors que `data/ui/*.fr.json` existe (Audience). Textes `objectives.rs` toujours codés (restes d'XP-AUDIENCE).
- Voir lot **L-F**.

**T-8. Compilation et itération : disque à 98 %, ~270 Go de `target/`, file d'attente `heavy.sh` permanente, durées des commandes lourdes non disponibles.**
- `df` : 25 Go libres sur 926 (98 %). `du -sk` des `target/` : checkout principal `client/target` 76 Go + `core/target` 8,7 Go ;
  9 worktrees client entre 1,8 et 40 Go (a00c : 40, a037 : 35, a956 : 28, a3bc : 21, a65d : 13…), `core/target` de 0,2 à 6,5 Go chacun :
  ≈ 270 Go apparents (partiellement partagés par clones APFS, `cp -c`). Le prompt de session interdit de relancer des builds sans
  nécessité ; les worktrees de MEM3/FPS en gardent ~43 Go pour une « fenêtre machine calme » qui n'arrive pas.
- `tools/heavy.sh` : 3 emplacements, scrutés toutes les 5 s sans file équitable (pas de FIFO : un lot court peut passer avant un lot
  attendu depuis longtemps). Pendant cet audit un `cargo run --example turn_perf` est resté en file > 5 min.
- Les durées des commandes lourdes (`out/heavy-log.tsv`) n'existent pas dans `fusion-a` : `tools/warm_worktree.sh` et la journalisation
  de `heavy.sh` sont dans `main` (554d924e9), pas dans `fusion-a`. **Je ne peux donc pas chiffrer les temps de compilation** ; ce
  constat est lui-même à corriger à la fusion.
- Profils : `core/Cargo.toml:46` `incremental = false` en dev (chaque retouche de `sim-campaign`, 74 k lignes à `opt-level = 2`, recompile le
  crate entier) ; `client` : `opt-level = 1` + dépendances à 3 (cohérent). Pas de `rust-lld`/`mold`, pas de `cargo-nextest`.

**T-9. Validation des schémas : pas de contrôle automatique « donnée ↔ schéma », 186 fichiers sans schéma identifiable.**
- 289 schémas dans `data/schemas`, 2 412 JSON de données (hors carte), aucun champ `$schema` dans les données, aucun test générique
  Python ou Rust qui valide tout `data/` contre `data/schemas/` (seuls des tests ciblés : `test_siege_works_schema.py`,
  `test_transitions_ui_schema.py`, `test_battle_replay_schema.py`, …). Le vrai contrôle est `serde` + `deny_unknown_fields`
  (344 occurrences dans `data-model`), qui n'attrape ni bornes ni énumérations ni pattern.
- Passe de validation à la volée (`jsonschema` 4.26, appariement par nom `X_rules`, `X`, répertoire) : 2 226 fichiers appariés, 186 sans
  schéma trouvé (`art` 58, `technologies` 45, `fx` 35, `ui` 11, `landmarks_v2` 8, `ai` 5, `battle_abilities` 5, `naval` 4, `rules` 6…) ;
  **11 fichiers invalides** contre le schéma supposé (`naval` ×2, `fx/battle_impact`, `fx/siege_fire`, `settlements`, `battle_maps`,
  `voice`, `movement`, `ai/diplomacy`, `ai/feudal`, `rules/siege_engines`). Appariement heuristique : les 11 sont à vérifier, certains
  seront de faux positifs ; c'est précisément l'absence d'un manifeste explicite qui empêche de trancher.
- Voir lot **L-F**.

### Mineurs

- **T-10. Stabilité : propre en surface.** Hors tests : `unwrap()` 0 dans `sim-campaign`/`sim-battle`/`game-api`/`data-model`, 2 dans `ai`, 15 dans
  `client/src`. `expect(...)` : 270 dans `sim-campaign` (22 dans `negotiation/apply.rs`, 14 dans `diplomacy/war.rs`, 13 dans `dynasty.rs`, 11 dans
  `battle_request.rs`), 38 dans `sim-battle`, 51 dans `ai` (surtout `examples/`), 81 dans le client. Le motif est `expect("checked")`,
  `expect("giver")` après un test préalable (`negotiation/apply.rs:70-101`) : couplage temporel fragile, pas une panique sur donnée. Aucun
  lint `clippy::expect_used` / `unwrap_used` dans `core/Cargo.toml`. 5 `panic!/unreachable!` dans `sim-campaign`, 12 dans `sim-battle` (dont des
  exemples). Une sauvegarde corrompue ou une donnée modifiée par un modder passe par `Result` dans `data-model` (4 `expect` seulement).
- **T-11. Duplication d'outillage de mesure.** Trois jeux d'outils mémoire (`memory.rs` du client, `tools/bevy_bench/footprint_parse.py`,
  `vmmap_regions.py`, `memlog_view.py`, et dans MEM3 `mem_report.py`, `metal_trace_*.py`) et deux bancs (`bench.sh`, `ab.sh`, `ab_multi.sh`
  sur FPS). À consolider dans un seul protocole (L-A).
- **T-12. `uisound_script*.txt`, `proof_*.txt`, `integ_script*.txt` sont dans `client/src/` (scripts de test mélangés aux sources).** Les déplacer
  sous `client/tests/scripts/`.

## Ce qui va bien (à ne pas casser)

Séparation core/client tenue au niveau des types (`game-api` sans règles), simulation déterministe, mimalloc (ADR 0776) déjà gagnant,
`gpu_pacing` (limite d'images en vol) qui a supprimé l'oscillation 3,8-6,5 Go de la bataille, banc de systèmes `profile`, dumps d'état
filtrés, 1 543 tests client, `deny_unknown_fields` partout dans `data-model`.

## Lots proposés

Ordre conseillé : **L-A → L-B / L-C en parallèle → L-D → L-E → L-F**. L-A d'abord, parce qu'il rend les autres mesurables sans machine calme.

### L-A. PERF-COMPTEURS : mesurer le travail, pas le temps (remplace perf_guard en wall-clock)
- Spec : un banc « travail par image » indépendant de la charge. (1) Compteurs lus dans le dump et le rapport de banc : draw calls et
  passes par vue (Bevy 0.20 : `RenderDiagnosticsPlugin` / `render_dev_tools`, déjà dans la feature `bevy-devtools`), pipelines créés (par image et
  cumulés, `PipelineCache`), entités visibles, triangles ou instances soumis, octets uploadés par image ; (2) temps CPU du fil principal et du
  fil de rendu en `thread_cpu_time` (comme `turn_perf --sequential`), pas en temps mural ; (3) mémoire : `footprint` + postes MEM5 (GPU non
  mappé, tas) dans le même rapport ; (4) `perf_thresholds.json` v2 : seuils en compteurs et en CPU-ms, calibrés par 3 mesures, plus
  `max_load` supprimé ; (5) `perf_guard.sh` obligatoire dans `full_check.sh` (le garde doit échouer quand il le faut).
- Critères mesurables : le rapport contient `draw_calls_p50/p95` et `pipelines_created` pour carte (pan, descente) et bataille (calme,
  mêlée) ; deux exécutions à des charges 5 et 30 donnent des écarts de compteurs ≤ 3 % et de CPU-ms ≤ 15 % ; le garde échoue si on
  réintroduit à la main un coût connu (ex. `CENT_ANS_VOL_STEPS=128`) ; seuils mémoire ramenés à la mesure réelle de la fusion + 5 %.
- Fichiers : `client/src/agent/{bench.rs,bench_battle.rs,perf_probe.rs,memory.rs}`, `tools/bevy_bench/{perf_guard.sh,perf_thresholds.json,bench.sh}`,
  `tools/full_check.sh`, `docs/performance-memoire.md`, ADR.
- Coût : 1 agent Sonnet, ~1 journée de lot, sans dépense cloud ; machine chargée acceptable (c'est le but).
- Sérialisation : aucun conflit de zones avec les lots visuels (touche `agent/` et `tools/bevy_bench/`) ; à faire avant L-B, L-E. Reprend les outils de
  MEM3 (`memory.rs`) et de FPS (`ab_multi.sh`) : rebaser ces deux branches dessus et les fermer.

### L-B. MEM6 : campagne sous 3 Go hors Metal, croissance de descente bornée
- Spec : (1) protocole `mem_protocol.sh bench` avec `vmmap`/`footprint` avant et après chaque descente pour attribuer les ~7 Go de croissance
  (caches de sol, détail, végétation, relief, tas de décodage) ; (2) caches bornés (taille en octets, LRU, valeurs dans `data/fx/`) ; (3)
  pré-cuisson KTX2/BC7 des feuilles de sol de campagne (supprime les décodages 185 Mo) ; (4) images listées au point 4 de `performance-memoire.md`
  en `RENDER_WORLD` seul (~100 Mo) ; atlas d'imposteurs et rasters du semis GPU seuls (≈ 900 Mo) ; (5) décharge des ressources de campagne
  à l'entrée en bataille (mesurer avant/après).
- Critères : campagne au repos ≤ 3 000 Mo de tas+CPU hors Metal (`footprint` − Metal alloué, mesure MEM5), empreinte de fin de banc ≤ repos + 1 000 Mo,
  bataille ≤ 4 500 Mo d'empreinte, pic de chargement ≤ 4 500 Mo ; temps d'image du banc non dégradés (compteurs L-A).
- Fichiers : `client/src/campaign/terrain/{ground_cache.rs,ground.rs,ground_detail.rs,lod.rs}`, `client/src/campaign/vegetation/`, `client/src/fx/`,
  `client/src/flow/loading.rs`, `tools/cent_ans_tools/ktx2_array.py`, `data/fx/` (+ schéma), `docs/performance-memoire.md`, ADR (0786 réservé par MEM3).
- Coût : 1 agent Sonnet pour la mesure, 1 pour les réductions ; ~2 builds release (~40-70 Go de `target` : réutiliser le clone APFS de MEM3).
- Sérialisation : touche la végétation et le sol de campagne = même zone que VIS-R3b/R4/R7 ; **après** leur fusion, ou en deux temps (mesure d'abord).
  Reprend la branche MEM3 (mesure déjà écrite).

### L-C. FORECAST-ASYNC : la prévision de saison hors du fil principal et incrémentale
- Spec : (1) mesurer la durée de `faction_forecast` (turn_perf `--sequential` + chronomètre dans `hud`) avant tout ; (2) déplacer l'appel dans une tâche
  `AsyncComputeTaskPool` (copie de l'état déjà faite par `simulate_next_turn`), résultat repris à l'image suivante ; invalider par `revision` ;
  (3) option cœur : prévision limitée aux grandeurs du joueur (n'exécuter ni l'IA ni les événements des autres factions si l'écart est négligeable,
  contrôlé par un test qui compare à la vraie fin de tour, comme `forecast_matches_the_real_end_of_turn`) ; (4) `season_story_with` hors fil principal
  dans la même tâche.
- Critères : aucun système HUD de campagne > 2 ms sur le fil principal au clic d'ordre (mesuré par le chronomètre `profile`), `forecasts_computed`
  inchangé, test de concordance prévision / fin de tour réelle vert, `smoke.sh` vert.
- Fichiers : `client/src/campaign/hud/mod.rs`, `core/crates/game-api/src/{hud.rs,causes.rs}`, `core/crates/sim-campaign/src/forecast.rs`.
- Coût : 1 agent Sonnet, ½ journée ; pas de build supplémentaire notable.
- Sérialisation : zone HUD campagne = XP-AUDIENCE, ECO-R1c (sim-campaign économie) : lancer entre deux lots d'économie ; faible risque de conflit en `forecast.rs`.

### L-D. BUILD-DX : profils, découpage du client, hygiène des `target/`
- Spec : (1) `[profile.release]` du client : thin LTO, `codegen-units` 1 ou 4 (A/B sur `turn_perf`, `battle_bench` et le banc mêlée en CPU-ms, L-A) ; si le gain
  < 3 %, le décrire et le refuser dans un ADR ; (2) profil `release-fast` (sans LTO) pour les bancs d'agents ; (3) client en workspace unique de plusieurs crates
  (`client-ui-kit`, `client-battle`, `client-campaign`, `client-app`) avec la feature `dev` propagée ; objectif : une retouche dans `battle/hud` ne recompile pas
  la campagne ; (4) `core/Cargo.toml` : `incremental = true` pour `sim-campaign` en dev (mesurer), ou découper `sim-campaign` (74 k lignes) ;
  (5) `tools/gc_targets.sh` : supprime les `target/` des worktrees fusionnés ou vieux de plus de N jours, avec garde `git merge-base --is-ancestor`, journal des Go
  libérés ; (6) file équitable (FIFO) dans `heavy.sh` et journal des durées (`out/heavy-log.tsv`) intégrés à `fusion-a`.
- Critères : durée `cargo check --features dev` après une retouche de `battle/hud/*.rs` réduite d'au moins 40 % (mesurée avant/après avec le journal) ; release
  client avant/après : CPU-ms de fin de tour et de mêlée (L-A) ; disque libre ≥ 120 Go après `gc_targets.sh` sur l'état actuel ; aucune commande lourde en file > 10 min
  quand une place se libère (FIFO).
- Fichiers : `client/Cargo.toml` (+ nouveaux `Cargo.toml`, déplacements de `client/src/*`), `core/Cargo.toml`, `tools/heavy.sh`, `tools/gc_targets.sh`, `CLAUDE.md`
  (commandes), ADR.
- Coût : refonte large (plusieurs jours de lot), 1 agent Sonnet pour le découpage mécanique (déplacements + `pub(crate)`), contrôle complet unique ensuite.
  Dépense de disque transitoire (deux builds complets).
- Sérialisation : **touche tous les fichiers du client** : à faire dans une fenêtre sans autre lot actif sur `client/` (entre deux itérations de fusion), avant les
  lots de rendu suivants. Le point (1) peut partir seul, immédiatement.

### L-E. FPS-MÊLÉE 60 : réglages en données, préchauffage vérifié
- Spec : (1) `volumetric_steps`, `shadow_distance_scale`, `shadow_map_size` par préréglage dans `data/fx/render_quality.json` (+ schéma) au lieu de variables d'environnement
  (reprend `CENT_ANS_VOL_STEPS`, `CENT_ANS_SHADOW_DIST`) ; (2) A/B en compteurs (L-A) : 64 → 40/32 pas, cascades ×1,0/0,7, ombre 4096, choix du meilleur rapport
  coût / rendu (une capture de contrôle) ; (3) `pipeline_warm.rs` vérifié (clippy, tests), activé pendant le voile de chargement, sondes factices pour les familles
  encore tardives (armes lâchées `instanced.wesl`, flipbook fx, `LineGizmo`, prepass pbr) ; (4) seuils `pipelines_created_after_veil` dans `perf_thresholds.json`.
- Critères : bataille : 0 pipeline créé après l'image 30 du premier tour joué (aujourd'hui 11-13) ; carte : 0 pic > 100 ms attribuable à `process_pipeline_queue_system`
  (aujourd'hui 350-1 100 ms) ; mêlée : CPU-ms de rendu −10 % et p50 ≤ 16,0 ms en banc calme unique (une mesure de confirmation, machine calme) ; draw calls de mêlée
  non supérieurs à la baseline L-A.
- Fichiers : `client/src/flow/pipeline_warm.rs` (branche FPS), `client/src/battle/decor/volumetric.rs`, `client/src/battle/scene/` (ombres), `client/src/settings/quality.rs`,
  `data/fx/render_quality.json` + `data/schemas/`, `tools/bevy_bench/ab_multi.sh`.
- Coût : 1 agent Sonnet, un seul lot sur machine calme pour la confirmation finale.
- Sérialisation : **après** L-A ; même zone que VIS-R1 (volumétrique/atmosphère), VIS-R4 (shaders de montagne) et les ombres : pas en parallèle avec eux. Un seul
  lot « shaders de bataille » à la fois.

### L-F. DATA-GARDE : manifeste de schémas, textes et constantes de rendu en données
- Spec : (1) `data/schemas/manifest.json` : chemin glob → schéma (explicite, relu par un test) ; test Rust `data-model` (ou pytest) qui valide **tout** `data/` contre
  son schéma et échoue sur un fichier sans schéma (liste d'exceptions justifiée) ; corriger les 11 fichiers invalides ou leurs schémas ; schémas des 186 fichiers
  non couverts (`technologies`, `art`, `fx`, `ui`…) ; (2) `GameData::load` parallèle (`std::thread::scope` ou `rayon`) avec mesure du temps avant/après ;
  (3) règles du client vers `game-api` : `army_cost` (custom battle), `total_cost/total_upkeep/can_seal` du panier de recrutement ; (4) textes : migrer par grappes les
  chaînes françaises de `core/` (`orders/error.rs`, `invariants.rs`, `capture.rs`, `ransom.rs`) et du client (`settings/rows.rs`, `keymap.rs`, `court/view.rs`) vers
  `data/ui/*.fr.json` avec clés, test « aucune chaîne française nouvelle hors data » en lint maison ; (5) constantes de rendu : tableau de bord `data/fx/*.json` pour les
  réglages les plus retouchés (`camera.rs`, `volleys.rs`, `skinned.rs`) ; lint clippy `expect_used` en avertissement dans `sim-campaign`, avec remplacement des 22 `expect("checked")`
  de `negotiation/apply.rs` par des types (`Checked<T>`) ou `Result`.
- Critères : `cargo test` échoue si un fichier de `data/` n'a pas de schéma ou n'y est pas conforme (0 fichier invalide, 0 sans schéma) ; `GameData::load` ≥ 2× plus rapide en
  temps CPU-mur sur machine non chargée (mesure `load_ms`, cible < 1 s en release) ; nombre de chaînes françaises dans `core/` −60 % sur les quatre fichiers visés ; zéro
  `expect("checked")` dans `negotiation/apply.rs`.
- Fichiers : `data/schemas/`, `core/crates/data-model/src/load.rs`, `core/crates/sim-campaign/src/{orders/error.rs,invariants.rs,capture.rs,ransom.rs,negotiation/apply.rs}`,
  `core/crates/game-api/`, `client/src/{menu/custom_battle.rs,campaign/panels/army/basket.rs,settings/}`, `data/ui/`.
- Coût : 2-3 agents Sonnet (schémas / textes / règles) ; mécanique, bien spécifiable ; une régénération de fixture auto-resolve si `data/unit_types` ou `data/rules/battle_*` bougent (ADR 0807).
- Sérialisation : le point (4) touche des fichiers actifs d'autres lots (`sim-campaign/orders`, `settings/rows.rs`) : par petites grappes, après chaque fusion d'itération ; (1)
  et (2) peuvent partir tout de suite (aucun conflit).

## Annexe : chiffres de base à reproduire

| Mesure | Valeur | Source |
|---|---|---|
| Carte, empreinte fin de banc | 12 539 Mo (pic RSS 9 101) | `out/bench/map_juge2full.json` |
| Carte, pan d30 p50 / p95 / max | 10,0 / 15,0 / 73,5 ms, 96,5 fps moyens | idem (machine chargée) |
| Carte, descente p50 / p99 / `worst_1pct_fps` | 11,5 / 34,5 ms / 25 fps | idem |
| Bataille, calme p50 / mêlée p50 / p95 / max | 12,5 / 16,9 / 18,0 / 40,3 ms | `battle_juge2battle.json` |
| Bataille, empreinte / pic RSS / entités max | 6 657 Mo / 3 672 Mo / 65 911 | idem |
| Seuils actuels | map 7 800, battle 4 500 Mo ; `max_load` 6 | `tools/bevy_bench/perf_thresholds.json` |
| Client / core | 251 584 lignes en 1 crate / 73 994 + 37 594 + 18 731 + 14 502 + 6 983 + 4 688 + 2 341 + 1 164 | comptage |
| `target/` | ~270 Go apparents, 25 Go libres (98 %) | `du`, `df` |
| `unwrap()` / `expect()` hors tests | cœur : 2 / 366 ; client : 15 / 81 | script de comptage |
| Fichiers de données sans schéma identifiable / invalides | 186 / 11 sur 2 412 | pass jsonschema à la volée |
