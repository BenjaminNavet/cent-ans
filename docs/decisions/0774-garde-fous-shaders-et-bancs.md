# 0774 — Garde-fous : shaders validés par test, journaux de shader fatals, bancs sur machine calme

Statut : accepté. Lot UX2 (juge iter-03, partie B et rapport technique).

## Contexte
Une erreur de shader de campagne (`coast_sampler` non déclaré dans `terrain.wesl`) a rendu le sol noir : wgpu journalise « failed to create shader module » puis le jeu continue, sans panique ni test rouge. Seul `ground_shader_validates` (bataille) existait, avec des stubs des imports Bevy écrits à la main. Les bancs de performance (`bench.sh`, `ab.sh`) se lançaient sans sémaphore ni attente de calme : mesures faussées par les autres sessions.

## Décision
- `client/src/shader_check.rs` (test `every_client_shader_compiles_and_validates`) : compile avec `wesl` chacun des .wesl de `client/src` (imports `bevy_*` réels lus dans les sources du registre cargo, module virtuel `constants` avec les valeurs de Bevy, `//@include hatch` de `provinces/drape.rs`), jeu de drapeaux d'un maillage complet (`VERTEX_*`, `DISTANCE_FOG`, `GROUND_CACHE`), puis valide avec naga. Une mutation (`terroir_sampler` remplacé par `coast_sampler`) rend le test rouge. `SHADER_CHECK_DUMP=<dossier>` écrit les WGSL compilés pour diagnostiquer.
- Un journal contenant `failed to create shader module`, `no definition in scope` ou `Shader compilation error` fait échouer `client/run_bg.sh` (code 1), `client/tests/smoke.sh` et `client/tests/shuffle.sh`.
- `tools/bevy_bench/bench.sh` passe par `tools/heavy.sh` (3 emplacements partagés) puis attend la calme (`tools/bevy_bench/calm.sh` : charge 1 min < `CALM_LOAD`=15 et aucun autre client `cent-ans`), avec un journal lisible (`calm: waiting 30s/1800s (load 41.7, limit 15; other cent-ans clients: 0)`). `ab.sh` construit via `heavy.sh` ; chacune de ses exécutions passe par `bench.sh`. `CALM_SKIP=1` désactive l'attente, `CALM_MAX_WAIT` borne (échec code 1 au-delà).

## Conséquences
- Un shader qui ne compile pas ne passe plus `cargo test` (≈ 0,5 s une fois compilé). Limite : un seul jeu de drapeaux ; une erreur propre à une autre combinaison de `shader_defs` n'est vue que par le journal du smoke.
- Un nouveau module Bevy (crate hors liste) doit être ajouté à la liste de `RepoResolver`.
