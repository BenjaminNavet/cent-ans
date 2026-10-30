# 0827. PERF-COMPTEURS : mesurer le travail, pas le temps

Statut : accepté (lot L-A du juge technique, 30/10/2026). Remplace les seuils en millisecondes de
`perf_guard.sh` (lot B5, ADR 0404/0748/0776) ; amende la section « Budget » de `docs/performance-memoire.md`.

## Contexte

Le garde de performance comparait des temps d'image (pan p50 20 ms, mêlée p95 25 ms) à des seuils mesurés sur machine
calme, et sautait (code 3) quand la charge dépassait 6. La charge de ce dépôt est de 20 à 42 en permanence : le garde ne
tournait jamais, et les seuils mémoire (7 800 / 4 500 Mo) étaient dépassés de 50-60 % sans signal (audit
`docs/audits/2026-10-30-juge-technique-iter3.md`, T-1, T-5). Bevy 0.20 n'expose pas de compteur d'appels de dessin, et
Metal n'a ni requêtes d'horodatage ni statistiques de pipeline : le banc ne pouvait pas dire « combien de draws ».

## Décision

1. **Compteurs de travail par phase** dans le rapport de banc (`work`) et le dump (`work`, `memory`), code dans
   `client/src/agent/counters.rs` :
   - appels de dessin, triangles, instances : la feature cargo `perf-counters` (bancs seulement, jamais le build
     joueur) active `bevy_render/detailed_trace`, qui fait émettre un événement `trace!` par appel de
     `TrackedRenderPass` ; une couche `tracing` les compte. Elle répond `Interest::never` à tous les autres sites
     d'appel du module (test statique dans la macro : pas de coût sur `set_bind_group`, etc.) ; les 8 lignes des
     événements `draw*` de `bevy_render =0.20.0` sont une constante datée (un changement de version les relit ; un
     décalage donne `null` et le garde échoue) ;
   - pipelines créés et total (`PipelineCache`, échantillonné chaque image dans le monde de rendu) ;
   - CPU en ms par image : processus entier (`getrusage`) et fil principal (`CLOCK_THREAD_CPUTIME_ID`) ; avec la feature
     `profile`, CPU du fil par système (`cpu_*_ms`). Un fil qui attend une machine chargée ne consomme pas de CPU ;
   - passes : `RenderDiagnosticsPlugin` (temps CPU par passe sous Metal), seulement en banc ou avec `CENT_ANS_COUNTERS=1` ;
   - mémoire dans le même rapport : empreinte, pic RSS, allocation Metal, tas mimalloc (courant, pic), empreinte hors tas.
2. **`perf_thresholds.json` v2** : seuils en compteurs et en CPU-ms par phase, écrits par `perf_calibrate.py` depuis
   3 mesures (pire des 3, + 5 % sur les comptes, + 15 % sur le CPU), valeurs de `reference` (médianes) conservées ;
   seuils mémoire = mesure + 5 %. Plus de `max_load`.
3. **`perf_guard.sh` ne saute plus** (`CALM_SKIP=1`) ; les temps d'image sont affichés, jamais comparés. Il est lancé par
   `tools/full_check.sh`.

## Conséquences

- Les lots suivants (MEM6, FPS mêlée, build) se mesurent en `draw_calls`, `pipelines_created`, CPU-ms : écarts entre deux
  exécutions à charges différentes bornés par le critère du lot (comptes ≤ 3 %, CPU ≤ 15 %).
- Le comptage coûte du CPU côté rendu (un événement par appel de dessin) : les valeurs de référence l'incluent ; les temps
  d'image d'un build joueur sont un peu meilleurs. Le build des bancs diffère du build joueur par cette feature.
- Non couvert : octets envoyés au GPU par image (aucun crochet sans patcher wgpu) ; draws indirects comptés pour un appel
  chacun, sans leurs sous-appels ; sous macOS les passes n'ont pas de temps GPU.
- Reprend les mesures mémoire de MEM3 (`memory.rs`) : à rebaser sur ce lot ; `ab_multi.sh` de la branche FPS compare désormais
  des champs `work`.
