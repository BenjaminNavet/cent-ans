# WIP — H8 rédaction massive du Codex

Conception : `docs/design/2026-09-23-histoire-et-savoir.md` §1 et §5.3. Branche : worktree agent H8.

## Compteur
- Fiches écrites (H8) : 0 / ~120

## Familles
- [ ] `_todo.md` (hors cuisine et médecine)
- [ ] Personnages
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

## Prochaine étape
Rédiger les fiches de `_todo.md`.
