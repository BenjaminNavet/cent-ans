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
- (14/10) `erode.py` : sur-échantillonnage bicubique x4, bruit guidé par la pente (+ plancher d'altitude : les plateaux d'auteur
  sont presque plats), comblement de cuvettes (priority-flood), loi de puissance implicite (numba), thermique, ancrage basse
  fréquence (sigma 5 px carte), cols protégés, mélange des blocs 3072+256 par rampe. Essai Alpes 360 px : ravines ramifiées, OK.
- Pyramide E0 (érodé) / E1 / E2-E3 (synthèse par tuile E1 là où relief >= 180 m) dans `data/map/pyramid/` + manifeste
  `relief_pyramid.json` (`e0_dir`). Game : `relief_pyramid.gd` (e0_dir, dir ".") et `terrain_builder.gd` (carte composée : pyramide
  si le manifeste existe, sinon détail procédural C5).
- Masques `compose/derived/erosion_masks.png` (R roche, G éboulis, B neige) lus par `masks.load_all` (roche -> splat via apply_cover).
- Tests `tools/tests/test_compose_erode.py` (6).

## Prochaine étape
- Implémenter `erode.py`, planche Alpes avant/après.
