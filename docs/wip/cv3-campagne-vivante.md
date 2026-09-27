# CV3 — Campagne vivante (orchestration)

Spec : `docs/design/2026-09-27-campagne-vivante.md`. Plan détaillé : voir la section « Lots » ci-dessous.
ADR réservée : `docs/decisions/0094-postures-embuscade.md`. Captures : `docs/img/cv3/`.

## Lots
| Lot | Contenu | Vague | État | Note wip |
|---|---|---|---|---|
| CV3-0 | 10 défauts de la carte de campagne (annexe A de la spec) | 1 | fusionné c4c86f66 (Île-de-France non reproduit, à revoir en partie pilote) | `cv3-0-carte.md` |
| CV3-1 | Postures (embuscade, marche forcée, camp retranché) + résultats nuancés (core) | 1 | fusionné 5096da0b | `cv3-1-postures.md` |
| CV3-3 | Moteur des rencontres + schéma + ~12 rencontres sourcées | 1 | fusionné 348fe5a7 | `cv3-3-rencontres.md` |
| CV3-2 | Ouverture d'embuscade dans sim-battle + libellés Godot | 2 | fusionné d508bd30 (palissade/badge à juger à l'œil) | `cv3-2-embuscade.md` |
| CV3-4 | UI campagne (boutons de posture, rencontres, bandeau de résultat) | 2 | lancé | `cv3-4-ui.md` |
| CV3-5 | Zone atteignable deux tons + lord à l'échelle TW | 2 | fusionné 0765207a | `cv3-5-zone-lord.md` |
| CV3-6 | IA postures/rencontres, équilibrage, sondes EQ6 | 3 | attente | |

## Choix tranchés
- Couvert d'une case : `CoverMap` (data-model) depuis `forest_kind.png` + `wetlands.json`, repli `Terrain` de province.
- Compétence d'embuscade = max(Commandement, intrigue de `character_effects`) — pas de branche Intrigue.
- Embuscade déclenchée à l'arrêt ZdC (`march::simulate`) → `movement::fight` avec `BattleOpening::Ambush`.
- Ordre `set_stance` existant étendu (pas de nouvel ordre) ; `get_stance_options(army)` exposé par le core pour l'UI.
- Classification des résultats branchée dans `apply_battle_result` (point commun auto/3D).

## Prochaine étape
Attendre CV3-4, fusionner, puis vague 3 (CV3-6 IA + équilibrage). CB (autre session) attendait la fusion de CV3-2 : faite.
