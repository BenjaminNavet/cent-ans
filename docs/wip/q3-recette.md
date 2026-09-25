# WIP Q3 — recette « comme un joueur » après ~10 sessions fusionnées

Branche `worktree-agent-aaf83c5a43e966bec`, base main 9fe0550a. Rapport : `docs/audit/q3-recette.md`,
captures `docs/audit/captures/q3/` (JPEG 1280 px, script de copie dans le scratchpad).
Pilote : `game/tests/q3_playtest.gd` (dérivé de Q1 ; phases à la carte, voix mesurées sur le bus
« Voix », ordres d'attaque en bataille, molette, onglets cliqués).
Réglages : `settings.cfg` sauvegardé dans le scratchpad avant chaque partie, restauré après.
Pyramide ZG : lien symbolique `data/map/pyramid` → dépôt principal (ignoré par git) pour tester le zoom.

## État
- [x] build core + import
- [x] pilote Q3
- [x] France 1920×1080 : 12 tours + actions, édits, diplomatie, commerce, agent, panneaux, sauvegarde
- [x] France 1920×1080 (2e partie) : bataille, siège, zoom, réglages
- [x] Angleterre 1280×720 (1re partie) : bataille avec ordres d'attaque, siège 3D (assaut), naval
- [x] Angleterre 1280×720 (2e partie) : diplomatie, édits, actions, agent, commerce, zoom ZG, 6 tours, réglages
- [x] naval seul jusqu'à l'écran de fin
- [x] rapport `docs/audit/q3-recette.md`
- [x] merge main (dc2360c6, EP inclus) ; [ ] build + smoke

## Corrigé
- 544aec3c touche du commerce R → X (R = carte religieuse, jamais atteinte)
- 85e8efd9 bouton « Donner l'assaut » hors écran (étiquette de siège à largeur 1)
- 7cd65af0 chronique plus haute que l'écran ; 1b4dc2fe rapport de saison en 720p
- bb1c54ac bandeau de sortie sur l'écran de fin ; 6e28f35e barre d'agent ; 74d24f03 fin navale

## Prochaine étape
Smoke après le merge de main, puis rapport final à l'orchestrateur.
