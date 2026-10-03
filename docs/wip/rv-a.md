# RV-A — volume du relief (normales moins exagérées, micro-relief filtré)

Branche `feat/rv-a`, worktree `gp-rv-a`. Chantier parent : `docs/wip/rv-relief-vivant.md`.

## Diagnostic
- Ombrage : `shading_relief` 2.1 (régional) / 1.3 (près) sur le gradient de la surface affichée.
- Maillage : HB6 (ADR 0143) a mis un gain local NÉGATIF (`gain_far` −0.4, `gain_near` −0.1,
  `res://resources/relief_exaggeration.tres`) : collines aplaties de 40 % dans le maillage, puis
  ombrage ×2.1 → relief lu à ≈ ×5.4 par les normales sur un volume ×2.6 = estompage IGN.
- Micro-relief : gradient à l'empreinte du pixel écran + détail fin `rl_relief` (relief_shade,
  1,5 m/niveau) → grain de papier en vue régionale.

## Plan
1. Shader : gradient basse fréquence (heightmap à `relief_coarse_mult` × l'empreinte) mélangé au
   gradient fin, part du fin `relief_fine_far` en vue régionale, 1 de près (quadtree fin intact).
2. `shading_relief` ≈ 1.15, `shading_relief_near` ≈ 1.0.
3. Volume : `gain_far` −0.4 → 0 (collines au relief ×4,31 du maillage stratégique), montagnes
   inchangées (écrasement SZ1 indépendant).

## État (terminé, à intégrer dans feat/rv)
- `terrain.gdshader` : `shading_relief` 2.1 → 1.15, `shading_relief_near` 1.3 → 1.0 ; nouveaux
  uniforms `relief_coarse_mult` 4, `relief_fine_far` 0.35, `relief_fine_near_footprint` 0.3,
  `relief_fine_far_footprint` 1.2 (gradient d'ombrage filtré ; pente des matériaux inchangée).
- `campaign_relief.gdshaderinc` : `campaign_display_gradient_pair` (champs lus une fois).
- `relief_exaggeration.tres` : `gain_far` −0.4 → 0 (`gain_near` −0.1 inchangé).
- Smoke OK, `zg8_relief_test` OK. Captures (`ss_shot.gd`, « avant » rejoué par `--param`/`--relief`) :
  grain de papier disparu en vue régionale, relief encore lisible (Massif central, Alpes) ; de près
  l'ombrage du sol nu est plus doux.
- Constaté, hors lot : densité d'arbres affichée et bandes de brume blanche varient d'un lancement
  à l'autre de `ss_shot.gd` (streaming / météo), sans lien avec le gain.

## Prochaine étape
Juger avec le soleil RV-B (plus rasant) : si le relief devient trop doux, remonter `shading_relief`
vers 1.3 ou `relief_fine_far` vers 0.5 plutôt que de revenir à 2.1.
