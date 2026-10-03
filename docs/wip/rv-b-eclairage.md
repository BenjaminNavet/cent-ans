# RV-B — éclairage de jeu de la carte (soleil, ambiance, perspective aérienne)

Lot du chantier RV (`docs/wip/rv-relief-vivant.md`), branche `feat/rv-b`, worktree `gp-rv-b`.

## Constat (03/10)
- Le `Sun` de `campaign_map.tscn` est à 45°, mais `CampaignAtmosphere` le repose après chargement
  depuis `data/fx/atmosphere.json` (`campaign.seasons`, 18-21°, azimut 240-250) : le soleil réel
  est déjà rasant. Ce qui reste « carte » : ambiance unique chaude (0.9, 0.85, 0.76) × 0.55, donc
  versants à l'ombre de même teinte que ceux au soleil ; soleil identique à chaque tour ; saut net
  au changement de saison ; brouillard de profondeur indépendant de l'inclinaison.

## Plan
- `data/fx/campaign_lighting.json` (+ `data/schemas/fx_campaign_lighting.schema.json`, test
  `tools/tests/test_campaign_lighting_schema.py`) : ambiance ciel froide, exposition, variation par
  tour (début / fin d'après-midi), durées de transition, perspective aérienne selon l'inclinaison.
- `game/scripts/visual/campaign_lighting.gd` (fonctions pures) ; `CampaignAtmosphere` reste seul
  propriétaire du soleil et interpole vers la cible (pas de second nœud qui se battrait avec
  `TurnLight` / la météo).

## État
- Squelette.

## Prochaine étape
Implémenter, captures avant/après (hc_shots, ≤ 4).
