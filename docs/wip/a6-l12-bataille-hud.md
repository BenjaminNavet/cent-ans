# A6-L12 : HUD et caméra de bataille (U22, U23, B2, B3, B4)

État : implémenté, tests headless à confirmer (cb5_alerts, u22, b4, smoke).
- U22 : harangue en haut au centre (`BattleSpeech.SUBTITLE_TOP`) ; zones du HUD ajustées (`BattleHud._fit_zones`) ; journal déjà replié par défaut (VN4) ; test `u22_hud_overlap_test.gd`.
- U23 : alertes par unité et type, rafraîchies, 10 s (`battle_alerts.json`), « ×9+ ».
- B2 : ombres de nuages (`battle_staging.json` cloud_shadows : tuile 2400 m, bruit lissé, plus rares).
- B3/B4 : `data/ui/camera_feel.json` bloc battle (`opening_*`, `marker_*`).
Ouvert : variation de sol macro de `battle_terrain` codée en dur dans le shader (pas de réglage).
