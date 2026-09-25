# B9b — fiches Codex, sujets 10 à 18 de l'audit § 10

Branche : `worktree-agent-afbe783dbb349d1a1` (partie de `main` 580fc208).

## État : terminé

- [x] 10 `cdx_marguerite_maultasch` (entité `prov_tirol`)
- [x] 11 `cdx_adolphe_de_la_marck` (entité `prov_liege`)
- [x] 12 `cdx_chateau_gaillard`
- [x] 13 `cdx_palais_des_papes` (alias « Palais des Papes » repris de `cdx_papaute_avignon`)
- [x] 14 `cdx_siege_de_dunbar` (alias « siège de Dunbar » repris de `cdx_black_agnes`, entité `evt_siege_de_dunbar`)
- [x] 15 `cdx_cassel` (alias « Cassel », « bataille de Cassel » repris de `cdx_louis_de_nevers`)
- [x] 16 `cdx_pont_saint_esprit`
- [x] 17 `cdx_jean_de_beaumanoir` (alias « Jean de Beaumanoir », « Beaumanoir » repris de `cdx_combat_des_trente`)
- [x] 18 `cdx_ordre_de_montesa`
- [x] liens `[[cdx_…]]` dans les descriptions (personnages, événement du combat des Trente, unité goedendag, fiches voisines)
- [x] rapport d'audit § 10 mis à jour

## Notes

- Les descriptions de provinces, colonies et factions ne passent pas (à ma connaissance) par
  `CodexText.format` : pas de `[[…]]` ajouté là, l'auto-lien par alias suffit.
- Validation : `uv run --project tools python` + `validate_codex(Path("data"))` (397 fiches,
  0 erreur), `uv run --project tools pytest tools/tests/test_codex.py` (3 passés). Aucun build.

## Prochaine étape

Fusion par l'orchestrateur (conflits possibles avec B9a sur `docs/histoire/audit-2026-09-25.md` § 10).
