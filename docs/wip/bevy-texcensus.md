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
