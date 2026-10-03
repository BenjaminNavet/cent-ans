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

## État (03/10) : fait, non fusionné
- `CampaignLighting` (fonctions pures) + `CampaignAtmosphere` : cible = soleil de saison
  (`atmosphere.json`, inchangé : bande 18-21° de l'ADR 0156, testée par `tb6_light_test`) modulé par
  la phase du tour (±3° de hauteur, ±15° d'azimut, plus chaud et −5 % d'énergie en fin
  d'après-midi ; bornes 15-28°) ; interpolation exponentielle (saison 8 s, tour 5 s), en pause
  pendant le soir doré de `TurnLight` ; posée d'emblée avant le chargement de la carte.
- Ambiance froide (0.54, 0.65, 0.92) × 0.56, part du ciel 0.2 (automne et hiver surchargés),
  exposition 0.98 (0.9 avant) pour garder la clarté.
- Perspective aérienne : brume de saison mêlée à 50 % vers (0.56, 0.68, 0.90) ; selon le tangage
  (32° → 60°) densité 0.85 → 0.5, perspective aérienne 0.6 → 0.12, début 1.25 → 1.9 × distance,
  fin 5.5 → 8. La vue régionale (tangage ≈ 34°) est donc « basse » : voile bleuté vers le haut de
  l'écran seulement, le premier plan reste net.
- `--light-turn=N` force la phase (captures) ; `settle_light()` pose la cible (tests).
- Tests : `game/tests/rv_b_lighting_test.gd`, `tb6_light_test.gd` adapté, `po_grade_test`, smoke,
  `tools/tests/test_campaign_lighting_schema.py`.
- Captures (hc_shots, planches avant / après dans le scratchpad, non versionnées) : effet net mais
  sobre ; l'aspect « estompage » vient surtout des normales haute fréquence (RV-A).

## Points ouverts
- SSAO non modifié (rayon plafonné à 3, coût non mesuré : machine chargée) ; l'occlusion de vallée
  relève de RV-D.
- Le ciel HDRI et la LUT changent encore d'un coup au changement de saison (existant).
- Jugement à l'œil en session principale après intégration avec RV-A/C/D.
