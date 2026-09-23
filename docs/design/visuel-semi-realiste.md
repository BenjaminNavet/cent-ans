# Chantier visuel semi-réaliste (session 5)

ADR : `docs/decisions/0004-semi-realistic-art-direction.md`. Branche `visual`, fusion dans `main` après rebase.
Travail en parallèle des lots F1-F5/F7-F9 (session 4) ; le lot F6 (rendu de carte) est absorbé ici.

| Lot | Contenu | Fichiers |
|---|---|---|
| **V1 Lumière et post-traitement** | Environnements carte et bataille (AgX, SSAO, brouillard, ciel, ombres), anticrénelage, `VisualSettings` | `campaign_map.tscn`/`battle.tscn` (Environment, Sun), `project.godot` [rendering], `game/scripts/visual/` |
| **V2 Terrain de campagne** | Splatmap (tools/geo), textures PBR Poly Haven, frontières SDF, eau (profondeur, écume, reflets), fleuves affinés, côte, bruit des Alpes corrigé | `game/shaders/terrain*`, `water*`, `game/scripts/map/{terrain_builder,sea,coast_renderer,rivers_renderer,polyline_mesh}.gd`, `tools/cent_ans_tools/geo/`, `data/map/` (nouveaux fichiers) |
| **V3 Végétation et décor de carte** | Forêts, bosquets, champs en `MultiMesh` selon la splatmap ; marqueurs d'armée (bannière + effectif) ; villes à l'échelle | `game/scripts/map/{vegetation,army_marker*,city_markers}.gd` |
| **V4 Batailles** | Sol texturé, herbe animée, ciel, arbres, figurines détaillées animées par shader, murailles en pierre | `game/scripts/battle/{battle_meshes,battle_terrain}.gd`, `game/shaders/battle_*` |
| **V5 Modèles et interface** | Retexture des `.glb` (Blender), cadres parchemin | `tools/blender_scripts/`, `game/assets/models/` |

Critère de fin de chaque lot : `smoke.gd` vert, captures avant/après dans `docs/img/visuel/`, pas de régression
de performance notable (carte ≥ 60 i/s au zoom moyen sur le Mac de développement).
