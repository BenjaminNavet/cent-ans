# Lot RS-A — Codex (données seulement)

Branche : `feat/rs-a-codex`, worktree agent. Lot de données : pas de build cargo.

## À faire
1. H10 lots 7-8 : 8 fiches Codex manquantes.
2. Fiche de mécanique « Difficulté ».
3. Validateur Codex + pytest.

## État
- [x] 8 fiches écrites : cdx_menagier_de_paris, cdx_forme_of_cury, cdx_verjus, cdx_cervoise,
  cdx_tranchoir, cdx_uroscopie, cdx_saignee, cdx_hildegarde_de_bingen.
- [x] `data/codex/_h10_links.md` supprimé (plus aucun id en attente) ; `docs/wip/h10-codex-table-medecine.md`
  mis à jour (lots 7-8 cochés, chantier clos).
- [x] `pytest tools/tests/test_codex.py tools/tests/test_codex_homonyms.py` : verts après l'écriture
  des 8 fiches.
- [x] Fiche de mécanique « Difficulté » (`cdx_jeu_difficulte`), chiffres tirés de
  `data/rules/difficulty.json` (ADR 0037), validée par `validate_codex` et pytest.
- [ ] Re-vérification finale (pytest complet) + merge de `main` + rapport.

## Prochaine étape
Relancer `pytest tools` en entier, merger `main` dans la branche, relancer les tests, puis rendre
la main.
