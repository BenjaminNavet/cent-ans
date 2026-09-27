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
| AN1a | Mouvement secondaire en shader (surcots, caparaçons, bannières, crins), vent d'`atmosphere.json` | `battle_soldier_skinned.gdshader` (partie sommets), shaders d'étendards | cent-ans-dev (worktree) | **fusionné** (b410fd6b) |
| AN1b | Nouveaux clips : victoire, 2-3 attentes, parade, coup par-dessus, cheval cabré/trébuche, impacts variés ; branchement dans les jeux de clips | `battle_skinned.py`, `battle_fine.py`, textures d'os, sélection des clips en GDScript | cent-ans-dev (worktree) | **fusionné** (038ad974) |
| PR1 | Retexture PBR (Poly Haven CC0) des accessoires Quaternius à aplats | `game/assets/models/**` props, `third_party/**/SOURCE.md`, `CREDITS.md` | cent-ans-dev Sonnet (worktree) | **non fusionné** (sans effet visible) |

Règles : chaque agent fusionne `main` dans sa branche avant de rendre la main, n'écrit jamais
dans `main` ; l'orchestrateur fusionne (ff). Cible cargo privée par worktree, `godot --import`
après copie des dylibs. Au plus 3 captures par lot.

## Journal
- 27/09 : fichier créé, vague unique lancée (AN1a, AN1b, PR1).
- 27/09 : **AN1b fusionné** en ff (038ad974) : 16 clips (victoire ×4, attentes ×6, parade, taille par-dessus, impacts ×2, cheval cabré/trébuche), `clips[64]`, jeux à 8 clips. Smoke + `an1b_clips_test` OK sur main. Écran de fin retardé de 3 s (`VICTORY_HOLD`) pour voir l'acclamation : **validé par le joueur**. Restes : `docs/wip/an1b-clips.md`. AN1a (partie A de l'ADR 0096) doit fusionner main : conflit attendu sur l'en-tête du shader et l'ADR.
- 27/09 : **AN1a fusionné** en ff (b410fd6b) : ondulation surcots/caparaçons/queues/crinières/étendards selon vitesse + vent (`battle_finish.json` wind, amplitudes `atmosphere.json` `secondary_motion`), coût ≤ +1 % (médiane), `--no-an1a` pour A/B. Smoke, `an1a_motion_test`, `an1b_clips_test`, pytest atmosphère OK sur main. Reste : jugement en jeu des amplitudes par le joueur ; restes dans `docs/wip/an1a-mouvement-secondaire.md`. PR1 en cours.
- 27/09 : **PR1 rendu, non fusionné.** 28 accessoires du pack Quaternius Medieval Village retexturés (Poly Haven CC0, branche `feat/pr1-props-retexture` 6c30fad2), mais ce pack n'est référencé nulle part (`game/`, `data/`, `tools/` : 0 occurrence) : les accessoires réels viennent du kit procédural `building_kit.py`. La dette n° 7 de la bible était périmée. Fusionner ajouterait ~16 Mo sans effet ; branche supprimée sur décision du joueur (script récupérable : commit 6c30fad2).
- 27/09 : vague AN1 close ; worktrees et branches supprimés.
