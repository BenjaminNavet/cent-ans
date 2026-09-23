# WIP — interface « à la Total War » (session e3, parallèle à l'orchestrateur de finalisation)

Plan : `docs/design/2026-09-23-audit-ui-total-war.md`.

| Lot | État | Où | Notes |
|---|---|---|---|
| Audit (19 défauts) | fait | main `dc77b3d` | captures `docs/img/audit-2026-09-23/` |
| F10b ordres du chef (sim-battle, data/battle_orders, pont, `leader_orders_bar.gd`) | agent lancé | worktree agent | l'orchestrateur retient F5 jusqu'à la fusion |
| F10a composants HUD campagne (army_strip, general_seal, end_turn_cluster, news_letters, hud_preview) | agent lancé | worktree agent | nouveaux fichiers seulement |
| Lot A corrections (défauts 1-5, 10-15) | attend | — | après fusion F1/F2/F3 par l'orchestrateur (il préviendra) |
| Intégration F10a dans map_ui / campaign_map | attend | — | après fusion F2/F3 |
| F10c bannières 3D | attend | session `visual` | à négocier avec game-project-76 |

Coordination : orchestrateur = session game-project-2b ; refonte visuelle = game-project-76 (possède shaders, map rendering, battle_meshes/terrain).
Prochaine étape : rapports des deux agents → revue des captures → fusion dans main → prévenir 2b (F5 peut démarrer).
