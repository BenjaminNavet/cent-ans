# VQ cgrade — étalonnage unique et perspective aérienne de la campagne 3D

## État (17/10)
- Re-vérifié : la passe post-tonemap `terrain/post.rs` couvrait déjà contraste 1,06 / saturation 1,08 pour toute l'image, mais le grade de saison (saturation 0,77, contraste, lift, hautes lumières) restait dans `terrain.wesl` (`campaign_grade`) et dans `billboard.wesl` (`foliage_grade`) : mer, rivières, maquettes, armées, charrettes y échappaient.
- Fait : le grade de saison passe dans la passe post (`post.wesl`, `sync_grade`), appliqué à l'image entière ; le terrain et la végétation reçoivent `grade.w = 0` (`lighting.rs::apply_light`), donc plus de double étalonnage et aucun changement de `terrain.wesl`.
- Perspective aérienne : déjà portée (`weather/atmosphere.rs`, brume par pixel, densité par distance et inclinaison, `campaign_lighting.json > aerial`).

## Reste
- Captures et A/B de coût, réglages éventuels de la balance soleil / ambiance.
