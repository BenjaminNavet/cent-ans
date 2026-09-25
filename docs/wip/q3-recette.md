# WIP Q3 — recette « comme un joueur » après ~10 sessions fusionnées

Branche `worktree-agent-aaf83c5a43e966bec`, base main 9fe0550a. Rapport : `docs/audit/q3-recette.md`,
captures `docs/audit/captures/q3/`. Pilote : `game/tests/q3_playtest.gd` (dérivé de Q1).
Réglages : `settings.cfg` sauvegardé dans le scratchpad avant chaque partie, restauré après.

## État
- [ ] build core + import
- [ ] pilote Q3 (siège 3D, naval, diplomatie, édits, commerce, agent, zooms, voix, réglages)
- [ ] partie France 12 tours 1920×1080
- [ ] partie Angleterre 6 tours 1280×720
- [ ] revue des captures, rapport, petits correctifs

## Constats en cours
- MF1 (menu Filtres) et EP (épique) ne sont pas dans main (branches `feat/map-modes`, `integration/epic`).
- Touche R : `map_toggle_trade` et `map_mode_religion` sur la même touche physique ; la diplomatie
  consomme R en premier, la couche commerce n'est jamais basculée au clavier.

## Prochaine étape
Écrire le pilote Q3 puis lancer la partie France.
