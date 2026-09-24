# WIP — chantier visuel (session 5, orchestrateur game-project-76)

Plan : `docs/design/visuel-semi-realiste.md`. Branche `visual` (worktree `.claude/worktrees/visual`).

| Lot | État | Agent | Notes |
|---|---|---|---|
| V1 Lumière | carte faite (bataille : V4) | orchestrateur | AgX, SSAO, ombres adaptatives, brouillard de profondeur, flou maquette, MSAA 4× + FXAA |
| V2 Terrain | fait, fusionné (563da5a) | agent | splat, SDF frontières, PBR Poly Haven, eau, fleuves, relief ×4,3 |
| V3 Végétation | fait, fusionné | agent | forêts/haies MultiMesh, villes Blender PBR, marqueurs à étendard + plaque d'effectif |
| V4 Batailles | fait, fusionné | agent + orchestrateur | ciel, sol PBR, herbe, figurines animées, bannières, siège en pierre ; coût GPU ×2,5 (V4b) |
| V5 Modèles/UI | à faire | | |

Captures « avant » : scratchpad de session (à copier dans docs/img/visuel/).

## Journal
- 19 h 45 : V1 carte commité (d61132f). Agents V2, V4 puis V3 lancés dans leurs worktrees depuis `visual`.
- Contrat V2→V3 : `data/map/splat.png` RGBA (R prairie, G cultures, B forêt, A roche) ; échelle verticale unique dans `MapData`.
- Prochaine étape : fusionner visual-v2, puis v3 et v4 dans `visual` (tests + captures), rebase sur `main`, fusion dans `main`.
- V2 fusionné dans visual (smoke OK). V3 prévenu de fusionner visual.
- 23 h 30 → 0 h 10 : quota épuisé, V3 et V4 interrompus (commits wip intacts). 0 h 15 : repris.
- 5 h 30 : V3 et V4 fusionnés dans visual, main fusionné dans visual (conflit trivial battle_scene.gd), smoke 16/16 OK, pytest 65 OK.
- Prochaine étape : fusion de visual dans main (orchestrateur de main), puis V4b (finitions batailles) et V2b (finitions carte).
- d7336b5 : visual fusionné dans main par l'orchestrateur de main (smoke 16/16). V4b et V2b lancés depuis visual 7e91ce0.
- ~8 h 30 → 10 h 10 : quota épuisé, V2b et V4b interrompus ; repris à 10 h 15 (V4b : 65403e8 LOD ; V2b : semis des haies et champs en cours).
- V4b fusionné dans visual (81ffcbb) : LOD arbres/figurines/herbe, livrées variées, chevaux, rivière. 1160 soldats : 74 → 87-101 i/s ; 4800 : 54 → 73-90 i/s. Smoke 16/16. Terminé par l'orchestrateur (agent bloqué en fin de lot).
