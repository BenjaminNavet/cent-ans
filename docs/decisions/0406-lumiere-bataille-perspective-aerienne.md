# 0406 — Lumière de bataille : perspective aérienne et rapport soleil/ciel

Statut : accepté (17/10, lot vq-blight).

## Contexte
Les captures de bataille (16/10) montraient une lumière plate, un voile laiteux uniforme et un horizon délavé. Le brouillard exponentiel uniforme gommait aussi bien le proche que le lointain ; l'ambiante (ciel HDRI + lumière ambiante) était trop forte devant le soleil (≈ 3:1).

## Décision
- Météo claire : `FogFalloff::Atmospheric` de Bevy (extinction et diffusion par canal : le lointain bleuit, le proche reste net) piloté par `fog.clear.aerial` de `battle_decor_look.json` ; absent = ancien voile exponentiel (brouillard, pluie, neige inchangés).
- Soleil de midi plus bas et plus fort (30 deg, 2,2), ambiante réduite (`ambient_boost` 0,85, `ambient.lux` 150), contraste et saturation de l'étalonnage « clair » relevés.
- SSAO : le niveau vient du préréglage (`ssao_quality`, jusque-là ignoré), pas de `battle_lighting.json`.
- Aucune mise à l'échelle 3D n'existe côté Bevy : la bataille est en résolution native. Bloom, tonemapping (AgX) inchangés.

## Conséquences
- Coût : SSAO « high » au lieu de « medium » (seul poste notable) ; brouillard et lumières quasi gratuits. A/B sous charge ~90 non concluant (voir docs/wip/vq-blight.md).
- Les autres météos pourront recevoir un bloc `aerial` par donnée seule.
