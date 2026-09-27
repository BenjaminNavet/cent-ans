# AN1 — Animation vivante + PR1 retexture des accessoires (orchestration)

Demande du joueur (27/09) : « quels assets et animations rendraient le jeu plus beau de manière
efficace ? » → recommandation acceptée (« oui »). Coût prévu : **0 $** (shader, Blender, CC0).
Bible : `docs/design/2026-09-25-bible-da.md` (§ 6, § 10 dette n° 7). ADR réservée : **0096**
(AN1, mouvement secondaire + nouveaux clips).

## Constat de départ
- Déjà là : ~45 clips cuits en texture d'os (`human_clip_specs` dans
  `tools/blender_scripts/battle_skinned.py`), phase par soldat, clip tiré dans un jeu, fondu
  0,35 s (`battle_soldier_skinned.gdshader`), poussière, piétinement, ombres de nuages.
- Manque : mouvement secondaire (tissus, bannières, crins), victoire, variantes d'attente,
  parade, coup par-dessus, cabrage/trébuchement, variantes d'impact ; accessoires Quaternius
  en aplats (dette n° 7).

## Lots

| Lot | Objet | Propriétaire des fichiers | Agent | État |
|---|---|---|---|---|
| AN1a | Mouvement secondaire en shader (surcots, caparaçons, bannières, crins), vent d'`atmosphere.json` | `battle_soldier_skinned.gdshader` (partie sommets), shaders d'étendards | cent-ans-dev (worktree) | lancé |
| AN1b | Nouveaux clips : victoire, 2-3 attentes, parade, coup par-dessus, cheval cabré/trébuche, impacts variés ; branchement dans les jeux de clips | `battle_skinned.py`, `battle_fine.py`, textures d'os, sélection des clips en GDScript | cent-ans-dev (worktree) | lancé |
| PR1 | Retexture PBR (Poly Haven CC0) des accessoires Quaternius à aplats | `game/assets/models/**` props, `third_party/**/SOURCE.md`, `CREDITS.md` | cent-ans-dev Sonnet (worktree) | lancé |

Règles : chaque agent fusionne `main` dans sa branche avant de rendre la main, n'écrit jamais
dans `main` ; l'orchestrateur fusionne (ff). Cible cargo privée par worktree, `godot --import`
après copie des dylibs. Au plus 3 captures par lot.

## Journal
- 27/09 : fichier créé, vague unique lancée (AN1a, AN1b, PR1).
