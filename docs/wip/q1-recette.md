# WIP Q1 — recette et intégration (nuit du 25/09)

Branche `worktree-agent-a69d4c733574b0ae3`. Rapport : `docs/audit/q1-recette.md`, captures
`docs/audit/captures/q1/`.

## Outil
- `game/tests/q1_playtest.gd` : pilote une partie en fenêtre (clics et touches poussés dans le
  viewport, captures numérotées, durées de fin de tour, FPS). Phases : battle, siege, actions,
  panels, turns, save (`--phase=all` par défaut).
  `godot --resolution 1920x1080 --path game --script res://tests/q1_playtest.gd -- --out=<dossier> --faction=fac_france --turns=16`
- Le pilote modifie `user://settings.cfg` (qualité, taille d'interface, tutoriel) : sauvegarder
  le fichier avant, le restaurer après.

## État
- [x] menu → France → 16 tours (1080p), 10 tours (720p)
- [x] bataille depuis la carte, siège en résolution automatique, recrutement, construction, déplacement libre
- [x] panneaux (diplomatie, cour, fiche, technologies, agents, Codex, encyclopédie, objectifs, aide, modes de carte)
- [x] sauvegarde / chargement rapides, réglages, qualité, taille d'interface
- [x] Angleterre 6 tours
- [ ] édits, commerce : absents de main (C4/C5 restés dans integration/tw)
- 8 correctifs commités (voir le rapport), 14 défauts restants priorisés.

## Prochaine étape
Fait : main fusionné (0c082a82, conflits UI3 résolus), cargo fmt/clippy/test (518), build, import, smoke (24 OK), partie rejouée après fusion (menu MM1 pris en charge par le pilote). Reste : les défauts P1-P3 du rapport, à répartir.
