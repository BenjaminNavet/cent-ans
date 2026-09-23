# WIP — interface « à la Total War » (session e3, parallèle à l'orchestrateur de finalisation)

Plan : `docs/design/2026-09-23-audit-ui-total-war.md`.

| Lot | État | Où | Notes |
|---|---|---|---|
| Audit (19 défauts) | fait | main `dc77b3d` | captures `docs/img/audit-2026-09-23/` |
| F10b ordres du chef (sim-battle, data/battle_orders, pont, `leader_orders_bar.gd`) | **fusionné** | main `1177714` | F5 débloqué |
| F10a composants HUD campagne | **fusionné** | main `1177714` | non branchés |
| Lot A corrections | **fusionné** | main `1177714` | revenu net, journal, élision, opacité, id, confirmation sortie, l'Écluse |
| Intégration F10a dans map_ui / campaign_map | agent lancé | worktree agent | accesseurs stables pour le tutoriel F8 |
| F10c bannières 3D | proposé | session `visual` | textures par moi (banners.py), 3D par visual : réponse attendue |

Coordination : orchestrateur = session game-project-2b ; refonte visuelle = game-project-76 (possède shaders, map rendering, battle_meshes/terrain).
Prochaine étape : rapport de l'agent d'intégration → captures → fusion → prévenir 2b ; bannières selon réponse de 76 ; restyle compact des cartes de bataille à proposer à F5.
