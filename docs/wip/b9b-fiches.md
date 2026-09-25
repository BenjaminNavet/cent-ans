# B9b — fiches Codex, sujets 10 à 18 de l'audit § 10

Branche : `worktree-agent-afbe783dbb349d1a1` (partie de `main` 580fc208).

## État

- [x] 10 `cdx_marguerite_maultasch` (entité `prov_tirol`)
- [x] 11 `cdx_adolphe_de_la_marck` (entité `prov_liege`)
- [x] 12 `cdx_chateau_gaillard`
- [x] 13 `cdx_palais_des_papes` (alias « Palais des Papes » repris de `cdx_papaute_avignon`)
- [x] 14 `cdx_siege_de_dunbar` (alias « siège de Dunbar » repris de `cdx_black_agnes`, entité `evt_siege_de_dunbar`)
- [x] 15 `cdx_cassel` (alias « Cassel », « bataille de Cassel » repris de `cdx_louis_de_nevers`)
- [ ] 16 `cdx_pont_saint_esprit`
- [ ] 17 `cdx_jean_de_beaumanoir` (alias « Jean de Beaumanoir », « Beaumanoir » repris de `cdx_combat_des_trente`)
- [ ] 18 `cdx_ordre_de_montesa`
- [ ] liens `[[cdx_…]]` dans les descriptions (personnages, unités, fiches existantes)
- [ ] rapport d'audit § 10 mis à jour

## Notes

- Les descriptions de provinces, colonies et factions ne passent pas (à ma connaissance) par
  `CodexText.format` : pas de `[[…]]` ajouté là, l'auto-lien par alias suffit.
- Validation : `uv run --project tools python` + `validate_codex(Path("data"))`, puis
  `uv run --project tools pytest tools/tests/test_codex.py`. Aucun build (disque plein).

## Prochaine étape

Rédiger 16-18 (`cdx_pont_saint_esprit` est déjà cité par `cdx_palais_des_papes`), puis rapport d'audit.
Liens déjà posés : personnages (Philippe VI, Louis de Nevers, David II, Benoît XII, Montagu,
Agnès Randolph), unité goedendag, fiches Avignon, papauté, Benoît XII, Clément VI, Écosse,
Black Agnes, Louis de Nevers, goedendag, charge, château fort, Louis de Bavière.
