# VQ blight — lumière de bataille (17/10)

## État
- Squelette. Constat : aucune mise à l'échelle 3D (MetalFX) n'existe dans le client Bevy (`upscale_*` du JSON non lus) : la bataille est déjà en résolution native.
- `ssao_quality` des préréglages n'est pas lu ; la qualité SSAO vient de `battle_lighting.json` (medium).

## Prochaine étape
Capture de base battle_low / battle_default, puis réglages (soleil/ambiante, brouillard, SSAO, ombres, tonemapping).

## Mesures
(à venir)
