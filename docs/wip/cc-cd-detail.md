# CC CD — détail rapproché réaliste (érosion hors ligne + pyramide fine)

Branche `cc/cd` (worktree `game_project-cccd`), partie de `cc-carte-composee` (C2b fusionné, a85373338).
Contexte : `docs/wip/cc-carte-composee.md` § « Retour du joueur sur les premières captures ».

## Plan
1. `cent-ans geo compose-erode` (`tools/cent_ans_tools/compose/erode.py`, réglages `data/map/compose/erode.json`,
   schéma `compose_erode`) : sur-échantillonnage x4 de `heightmap.png` (E1 = 180 m), bruit fractal guidé par la pente,
   érosion fluviale (puissance de cours d'eau implicite, numba) + thermique, ancrage basse fréquence sur le relief d'auteur,
   cols protégés. `heightmap.png` n'est PAS modifié (navgrid inchangé).
2. Sortie : pyramide `data/map/pyramid/` (hors git) E0 (érodé), E1, E2-E3 synthétisés là où le relief le justifie ;
   manifeste `data/map/pyramid/relief_pyramid.json` ; changement minimal côté `game/`.
3. Masques roche / éboulis / neige (pente + incision) ; vérifier les textures TX sur la carte composée.
4. Tests pytest petite grille ; planches 2D `~/dev/cent-ans-raw/compose/cd/`.

## État
- (14/10) squelette : réglages + schéma.

## Prochaine étape
- Implémenter `erode.py`, planche Alpes avant/après.
