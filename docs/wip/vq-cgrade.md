# VQ cgrade — étalonnage unique et perspective aérienne de la campagne 3D

## État (17/10)
- Re-vérifié : la passe post-tonemap `terrain/post.rs` couvrait déjà contraste 1,06 / saturation 1,08 pour toute l'image, mais le grade de saison (saturation 0,77, contraste, lift, hautes lumières) restait dans `terrain.wesl` (`campaign_grade`) et dans `billboard.wesl` (`foliage_grade`) : mer, rivières, maquettes, armées, charrettes y échappaient.
- Fait : le grade de saison passe dans la passe post (`post.wesl`, `sync_grade`), appliqué à l'image entière ; le terrain et la végétation reçoivent `grade.w = 0` (`lighting.rs::apply_light`), donc plus de double étalonnage et aucun changement de `terrain.wesl`.
- Perspective aérienne : déjà portée (`weather/atmosphere.rs`, brume par pixel, densité par distance et inclinaison, `campaign_lighting.json > aerial`).

## Mesures
- Captures offscreen d=60 Paris et d=100 Beauce (hors dépôt) : rivières et maquettes suivent maintenant le sol (bleu moins saturé), sol inchangé à l'œil ; la brume (bleutée, débute à 1,25 x d, pente ajustée) est elle aussi étalonnée, comme dans Godot où l'ajustement porte sur tout l'environnement.
- Brume du matin / météo « fog » : voile plafonné à 0,07 et amincie de près (`weather_mist`), pas de voile laiteux uniforme à 30-120.
- A/B carte (`ab.sh map`, 3 paires, `CENT_ANS_PERF_OFF=post` contre défaut) : delta p50 +1,0 ms, p95 +10,6 ms en faveur du défaut, soit du bruit (machine à charge ~150) ; aucun coût ajouté (le grade déménage de deux shaders vers la passe existante, quelques ALU).
- Pas de réglage soleil / ambiance : l'ombrage du relief est lisible, rien de gratuit à gagner sans regard du joueur.

## Reste
- La brume passe désormais par le grade (saturation 0,77) : un peu plus grise qu'avant ; à juger au regard du joueur (`aerial.blue` dans `campaign_lighting.json`).
- Tonemapping : la caméra n'est pas HDR, le terrain n'est pas passé à AgX alors que les PBR (maquettes, armées) le sont dans leur shader ; non traité (changerait le terrain).
