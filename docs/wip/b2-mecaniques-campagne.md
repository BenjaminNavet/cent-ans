# WIP — B2 Mécaniques de campagne (fiches Codex `cdx_jeu_*`)

Spec : `docs/design/2026-09-25-bulles-partout.md` (lot B2). Branche : `worktree-agent-a1391ebbb72a033ca`.

## État
- [x] Collecte des chiffres (code `core/crates/sim-campaign`, `data/`)
- [ ] Fiches `cdx_jeu_*` (~25)
- [ ] `gameplay` ajouté aux fiches existantes (zone de contrôle, rançon, mutations, taille, gabelle, aides, Carême, chevauchée, ponts/gués)
- [ ] Liens `[[cdx_jeu_*]]` dans les données (édits, règles)
- [ ] Validateur Codex + pytest verts

## Fiches écrites
cdx_jeu_tresor, cdx_jeu_impot, cdx_jeu_entretien, cdx_jeu_edits, cdx_jeu_commerce, cdx_jeu_ordre_public, cdx_jeu_population, cdx_jeu_devastation, cdx_jeu_famine, cdx_jeu_table

## Prochaine étape
Écrire les ids restants listés dans `data/codex/_b2_links.md` (liste temporaire tolérée par le validateur ; à supprimer en fin de lot).

## Validateur
`uv run --project tools python -c "from pathlib import Path; from cent_ans_tools.codex import validate_codex; r=validate_codex(Path('data')); print(len(r.entries)); print('\n'.join(r.errors))"`
ou `uv run --project tools pytest tools/tests/test_codex.py`.
