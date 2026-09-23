# WIP — H2 Codex et bulles imbriquées

Conception : `docs/design/2026-09-23-histoire-et-savoir.md` §1. Branche : worktree agent H2.

## État
- [x] Squelette (API publique, fichiers vides)
- [x] Schéma `data/schemas/codex.schema.json` + `codex_id` commun
- [ ] 20 fiches (en cours, agent de rédaction) d'amorce `data/codex/` + `_todo.md`
- [x] Validateur `tools/cent_ans_tools/codex.py` + `tools/tests/test_codex.py`
- [x] `CodexStore`, `CodexText`, `CodexBubbles`, `CodexWindow`
- [x] Épinglage des infobulles F2 (touche T)
- [x] Hooks : fiche personnage, chronique, infobulles (techs)
- [x] Smoke `_run_codex` écrit (passe dès que les 20 fiches existent)
- [ ] Captures `_run_codex` + captures `docs/img/codex-*.png`

## Prochaine étape
Terminer et relire les fiches, faire passer pytest + smoke, captures.
