# WIP — interface « à la Total War » (session e3, parallèle à l'orchestrateur de finalisation)

Plan : `docs/design/2026-09-23-audit-ui-total-war.md`.

| Lot | État | Où | Notes |
|---|---|---|---|
| Audit (19 défauts) | fait | main `dc77b3d` | captures `docs/img/audit-2026-09-23/` |
| F10b ordres du chef (sim-battle, data/battle_orders, pont, `leader_orders_bar.gd`) | **fusionné** | main `1177714` | F5 débloqué |
| F10a composants HUD campagne | **fusionné** | main `1177714` | non branchés |
| Lot A corrections | **fusionné** | main `1177714` | revenu net, journal, élision, opacité, id, confirmation sortie, l'Écluse |
| Intégration F10a dans map_ui / campaign_map | **fusionné** | main `c532d40` | ArmyPanel retiré, cloche = alertes, accesseurs tutoriel ; capture `docs/img/hud-campaign.png` |
| F10c bannières 3D | textures **fusionnées** `6e82b24` | 3D : session `visual` (V3 carte, V4 bataille) | `cent-ans assets banners` |

Coordination : orchestrateur = session game-project-2b ; refonte visuelle = game-project-76 (possède shaders, map rendering, battle_meshes/terrain).
Prochaine étape : rien de bloquant côté ui-tw. Interface de bataille compacte = F5 (orchestrateur). Bannières 3D = session visual (fait sur la carte). Signalé à 2b : Philippe VI meurt avant 1340 dans la partie de capture « chronicle ».
