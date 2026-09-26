# Relecture historique — Orléans 1:1 (VH7)

Branche `feat/relecture-orleans` (worktree d'agent). Mission : relire les restitutions de
`data/landmarks_v2/orleans.json` (faits à relire de `docs/wip/vh7-orleans.md`), corriger d'après
des sources sérieuses, rapport `docs/histoire/relecture-vh-orleans.md`.

## État : terminé, à fusionner par l'orchestrateur
- [x] Sources : SAMO « Les enceintes urbaines » (2014/2023), Carron et Guillemard 2012
      (Archéopages 33), Petit 1987 (RACF 26), Inrap (atlas : enceintes, place De Gaulle, Martroi,
      notice « La Loire »), Jourd'heuil (base des collégiales), Alix et Noblet 2009, Wikipédia
      sourcée (Chenesseau, Collin, Journal du siège)
- [x] Corrections JSON : mur ouest de l'accrue et porte Renart (place De Gaulle), fossé 15 m,
      gabarit du castrum, axe du pont, bastille Saint-Antoine en bois (1417) + chapelle, boulevards
      1417 (Renart 1418), cathédrale romane à l'ouest du chevet (`sainte_croix_romane`, chantier
      supprimé), Augustins rasés en 1428 + bastille anglaise 1429, salle des Thèses 1415
- [x] Rapport `docs/histoire/relecture-vh-orleans.md`, section Orléans de `docs/landmarks-v2.md`
- [x] Tests : `test_landmarks_v2.py` (32 OK, faits corrigés figés), `vh4_landmarks_test` OK

## Prochaine étape
Fusion (fichiers touchés : `orleans.json`, `test_landmarks_v2.py` (test Orléans seulement),
`vh4_landmarks_test.gd` (un libellé), `docs/landmarks-v2.md` (section Orléans)). Point ouvert :
date de l'accrue (1345 gardé ; 1391 possible d'après Carron 2012).
