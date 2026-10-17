# VQ blight — lumière de bataille (17/10)

## Fait
- Constat : le client Bevy n'a aucune mise à l'échelle 3D (`upscale_*` de `render_quality.json` jamais lus) : la bataille est déjà en résolution native, rien à retirer.
- Constat : `ssao_quality` des préréglages n'était pas lu (le niveau venait de `battle_lighting.json`, « medium ») ; désormais `RenderQuality.ssao_quality` pilote la qualité (low/medium/high/ultra) et SSAO est bien actif par défaut (`cameras_with_ssao` = 1 dans le dump, hors écran compris).
- Perspective aérienne : `FogFalloff::Atmospheric` (extinction/diffusion par canal, lueur du soleil côté astre) pour la météo claire, réglée par `fog.clear.aerial` de `data/fx/battle_decor_look.json` (`distance_fog` dans `battle/decor/atmosphere.rs`). Absent = ancien voile exponentiel (brouillard, pluie, neige inchangés).
- Soleil de midi : élévation 36 -> 30 deg, énergie 1,75 -> 2,2 (ombres plus longues, rapport soleil/ciel plus réaliste) ; ambiante : `ambient_boost` 1,35 -> 0,85, `ambient.lux` 350 -> 150 ; étalonnage clair : contraste 1,05 -> 1,14, saturation 0,98 -> 1,04.
- Écartés/pas faits : bloom (reste coupé), changement de tonemapping (AgX gardé), PCSS/Temporal (pas de TAA en bataille), atmosphère Bevy physique (le ciel HDRI donne déjà le ciel et l'éclairage ambiant).

## Mesures
Machine chargée (~90) : 3 paires A/B alternées, ms/image (900 images, Crécy-like « Battle » par défaut) A 68,2 / 39,8 / 43,0 ; B 67,8 / 60,3 / 33,1 : bruit (±15 ms) > effet, aucun écart décelable. Surcoût attendu : SSAO « high » (échantillons) seul coût GPU notable ; brouillard atmosphérique et lumières = quelques ALU. À remesurer sur machine calme (banc vq-bench).

## Reste
- Banc sur machine calme ; essai SSAO « ultra » ; tonemapping/exposition à comparer à l'œil ; sol pâle (lot bdecor, non touché).
