# WIP — B8 Auto-lien sans homonymes

Branche : `feat/b8-homonymes` (worktree agent). Contexte : `docs/wip/bulles-partout.md`, vague 2.

## État
- [x] Alias le plus long prioritaire (tri déjà présent) ; limites de mot : le tiret entre deux
  mots lie (« Saint-Omer » ne lie pas « Omer »), l'apostrophe sépare (« d'Artois » lie « Artois »).
- [x] `exclude_contexts` : schéma, `CodexStore.is_excluded`, `CodexText._link_segment`, validateur + tests.
- [x] Échappement `[[!texte]]` : `CodexText.format` (rendu protégé par `[lang=fr]…[/lang]`), `plain`, validateur.
- [ ] Script d'audit `tools/cent_ans_tools/codex_homonyms.py` + test.
- [ ] Corrections des faux liens trouvés (liste ci-dessous).
- [ ] Smoke `codex_bubbles` : cas B8.

## Prochaine étape
Écrire l'audit, le lancer, corriger.
