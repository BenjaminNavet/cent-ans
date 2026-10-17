# VQ bench (banc bataille + A/B)

État : `--bench-battle` (client/src/agent/bench_battle.rs), `bench.sh --bench-battle`, `ab.sh`, README à jour.
Reste : voir mesures ci-dessous. Documentation : tools/bevy_bench/README.md.

## Mesures (17/10, release, machine très chargée load ~110, indicatif)
Crécy, 1 essai (`bench.sh vq1 --bench-battle`) : calm p50 17,2 / p95 55,1 ms (631 images) ; melee p50 35,6 / p95 103,7 ms (455 images), contact après 568 s simulées, 46/30 régiments. Chiffres gonflés par la charge : seuls les deltas A/B alternés comptent.
Non exécuté : `cargo test` (build de test tué par SIGTERM sous la charge) ; `clippy --all-targets -D warnings` vert, shellcheck propre. `ab.sh` non exécuté de bout en bout (même base que bench.sh).
