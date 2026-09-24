# WIP — H8 rédaction massive du Codex

Conception : `docs/design/2026-09-23-histoire-et-savoir.md` §1 et §5.3. Branche : worktree agent H8.

## Compteur
- Fiches écrites (H8) : 33 / ~120 (53 au total)

## Familles
- [x] `_todo.md` (hors cuisine et médecine)
- [ ] Personnages (en cours)
- [ ] Dynasties, lieux, factions
- [ ] Guerre (batailles, armement, chevalerie)
- [ ] Société, économie, institutions
- [ ] Religion et savoirs
- [ ] Héraldique, calendrier, vie quotidienne
- [ ] Liens `[[…]]` dans `data/characters`, `data/events`, `data/technologies`
- [ ] Onglets de `codex_window.gd`

## Constats
- Champs formatés par `CodexText.format` : `characters.description` (fiche personnage),
  `events.text` (fenêtre de chronique), `technologies.description` (infobulle riche).
- Non formatés : `factions.description` (non affichée), `events.title` et `options.text`
  (texte brut dans la chronique et le journal) → pas de liens.

## Notes de reprise
- Les ids cités mais pas encore écrits sont listés en fin de `_todo.md` (section H8), régénérée
  par un script (liens → ids manquants, ids écrits retirés).
- `tools/cent_ans_tools/codex.py` : les ids de tout `data/codex/_*.md` sont tolérés (règle de
  chevauchement limitée à `_todo.md`), comme annoncé par l'orchestrateur ; main n'avait pas encore
  ce changement au moment de la fusion.
- Ne pas rédiger les ids de `_diet_links.md` / `_herb_links.md` (lot dédié) : y lier seulement.

## Prochaine étape
Personnages restants (section H8 de `_todo.md`), puis dynasties, lieux, factions.
