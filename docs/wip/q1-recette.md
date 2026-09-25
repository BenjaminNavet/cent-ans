# WIP Q1 — recette et intégration (nuit du 25/09)

Branche `worktree-agent-a69d4c733574b0ae3`. Rapport : `docs/audit/q1-recette.md`.

## Outil
- `game/tests/q1_playtest.gd` : pilote une partie en fenêtre (clics et touches poussés dans le
  viewport, captures numérotées, durées de fin de tour, FPS).
  `godot --resolution 1920x1080 --path game --script res://tests/q1_playtest.gd -- --out=<dossier> --faction=fac_france --turns=12`

## État
- [ ] menu → France → tours
- [ ] recrutement, construction, déplacement, siège, bataille, auto-résolution
- [ ] diplomatie, commerce, édits, agents, technologies, personnages, Codex
- [ ] sauvegarde, chargement, réglages
- [ ] Angleterre
- [ ] 1280×720

## Prochaine étape
Premier passage menu → campagne → tours.
