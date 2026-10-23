# Solari campagne : écran de chargement bloqué (lot solari-camp-load)

Cause : sous Solari la caméra de carte devient HDR + `CameraMainTextureUsages::STORAGE_BINDING` + `Msaa::Off`.
Bevy clé la texture principale d'une vue par (cible, format, usages, msaa) : la caméra UI de la campagne
(Hdr absent, usages par défaut) obtenait sa propre texture noire et le blit final remplaçait le monde ;
seule l'UI (HUD + carte de prologue) restait visible. Même défaut que la bataille (8e0ce7c6d).

Correctif : `solari_proxy.rs` : `match_ui_camera` (Hdr, Tonemapping::None, usages, Msaa::Off dès que la
caméra de carte porte SolariLighting) et `release_ui_camera` à la sortie de la campagne.

Vérifié : capture 800 images Solari actif : carte visible. Clippy/tests OK ; bataille et campagne 300 images OK Solari coupé.
`enabled:false` conservé par défaut.
