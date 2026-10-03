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

## État
- Squelette ; import Godot en cours.

## Prochaine étape
Implémenter le shader, captures avant/après (ss_shot.gd), smoke.
