# Bevy texcensus — recensement des textures (lecture seule)

Statut (2026-10-22) : code écrit, pas encore compilé (banc release en cours, pas de cargo).

## Fait
- `client/src/agent/texture_census.rs` : sections de dump `textures` (Assets<Image> par préfixe, top 30, données CPU retenues)
  et `texture_loads` (durées mesurées aux sites de décodage / mips / atlas, drapeau `main`).
- Instrumenté : `strip_to_array` / `grid_to_strip` (decor/ground.rs), `fx/model_mips`, `water/detail`, `water::load_png`,
  `terrain/ground.rs` (`load_pack`, feuilles JPG, PNG 8 bits), héraldique, `load_ids_png`, `vegetation::build_atlas_mips`.

## Suite
1. Attendre la fin du banc, compiler, clippy, tests.
2. Crécy + campagne via run_bg.sh --offscreen, dump filtré `textures` + `texture_loads`.
3. Consigner les mesures et la recommandation ici.

## Blocage (2026-10-22)
- Disque plein (130 Mo libres sur 926 Go) : ~10 worktrees d'agents avec 11-23 Go de `client/target` chacun. Compilation impossible,
  code jamais compilé ni clippy. Les scripts de mesure sont prêts hors dépôt (battle: ouverture de Crécy puis `wait 900` + `dump`).
- Aucun banc `perf_` ne tournait : l'attente `pgrep -f "bench.sh perf_"` se détectait elle-même (la commande contient le motif).
  Utiliser `pgrep -f "[b]ench.sh perf_"`.
- Suite : libérer du disque (décision du lanceur), `cargo clippy --features dev --all-targets -- -D warnings`, `cargo test --features dev`,
  lancer Crécy et campagne avec `CENT_ANS_MUTE=1 client/run_bg.sh --offscreen --script ...`, remplir la section Mesures.
