# WIP — chantier visuel (session 5, orchestrateur game-project-76)

Plan : `docs/design/visuel-semi-realiste.md`. Branche `visual` (worktree `.claude/worktrees/visual`).

| Lot | État | Agent | Notes |
|---|---|---|---|
| V1 Lumière | carte faite (bataille : V4) | orchestrateur | AgX, SSAO, ombres adaptatives, brouillard de profondeur, flou maquette, MSAA 4× + FXAA |
| V2 Terrain | en cours | agent (branche visual-v2) | + exagération verticale ×14 à réduire |
| V3 Végétation | en cours | agent (branche visual-v3) | forêts MultiMesh, villes, marqueurs ; lit splat.png (contrat V2) avec repli procédural |
| V4 Batailles | en cours | agent (branche visual-v4) | inclut Environment de battle.tscn |
| V5 Modèles/UI | à faire | | |

Captures « avant » : scratchpad de session (à copier dans docs/img/visuel/).

## Journal
- 19 h 45 : V1 carte commité (d61132f). Agents V2, V4 puis V3 lancés dans leurs worktrees depuis `visual`.
- Contrat V2→V3 : `data/map/splat.png` RGBA (R prairie, G cultures, B forêt, A roche) ; échelle verticale unique dans `MapData`.
- Prochaine étape : fusionner visual-v2, puis v3 et v4 dans `visual` (tests + captures), rebase sur `main`, fusion dans `main`.
