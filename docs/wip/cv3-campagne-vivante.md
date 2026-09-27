# CV3 — Campagne vivante (orchestration)

Spec : `docs/design/2026-09-27-campagne-vivante.md`. Plan détaillé : voir la section « Lots » ci-dessous.
ADR réservée : `docs/decisions/0094-postures-embuscade.md`. Captures : `docs/img/cv3/`.

## Lots
| Lot | Contenu | Vague | État | Note wip |
|---|---|---|---|---|
| CV3-0 | 10 défauts de la carte de campagne (annexe A de la spec) | 1 | lancé | `cv3-0-carte.md` |
| CV3-1 | Postures (embuscade, marche forcée, camp retranché) + résultats nuancés (core) | 1 | fusionné 5096da0b | `cv3-1-postures.md` |
| CV3-3 | Moteur des rencontres + schéma + ~12 rencontres sourcées | 1 | lancé | `cv3-3-rencontres.md` |
| CV3-2 | Ouverture d'embuscade dans sim-battle + libellés Godot | 2 | lancé | `cv3-2-embuscade.md` |
| CV3-4 | UI campagne (boutons de posture, rencontres, bandeau de résultat) | 2 | attente CV3-1/3 | |
| CV3-5 | Zone atteignable deux tons + lord à l'échelle TW | 2 | lancé | `cv3-5-zone-lord.md` |
| CV3-6 | IA postures/rencontres, équilibrage, sondes EQ6 | 3 | attente | |

## Choix tranchés
- Couvert d'une case : `CoverMap` (data-model) depuis `forest_kind.png` + `wetlands.json`, repli `Terrain` de province.
- Compétence d'embuscade = max(Commandement, intrigue de `character_effects`) — pas de branche Intrigue.
- Embuscade déclenchée à l'arrêt ZdC (`march::simulate`) → `movement::fight` avec `BattleOpening::Ambush`.
- Ordre `set_stance` existant étendu (pas de nouvel ordre) ; `get_stance_options(army)` exposé par le core pour l'UI.
- Classification des résultats branchée dans `apply_battle_result` (point commun auto/3D).

## Prochaine étape
Attendre la fin de la vague 1, fusionner (worktree séparé, ff-only), réimporter Godot, lancer la vague 2.
