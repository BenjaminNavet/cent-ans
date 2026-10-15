# bm-bvis3 : ciel, lisière, horizon des batailles (client Bevy)

Branche `bm/bvis3` (suite de bvis2 ; fusionne bm/bvis2 et bm/siegemat).

## Fait
- Ciel : le panorama HDRI (cube de `SkyLightState`, 512 px par face, construit sur 6 fils) est le fond (`Skybox`, luminance = unité d'exposition) ; le dôme du décor ne sert plus que de repli (caché dès qu'un `Skybox` est posé). Les 9 ciels .hdr se chargent (feature `hdr` des défauts de Bevy) et le choix météo / saison / heure est celui de bvis2.
- Herbe : mesure A/B sans vent (`NotShadowReceiver` posé ou non) : l'herbe reçoit déjà les ombres (StandardMaterial, drapeaux du maillage) ; rien à corriger.
- Lisière proche (`trees.rs::plan_ring_woods`) : port de l'anneau proche Godot (grille 13 m, bruit des bois 1/420 m, seuils `near_woods` par terrain, buissons de lisière, arbres isolés), jusqu'à 900 m du champ.
- Imposteurs : au-delà de 300-400 m, houppiers remplacés par des cartes croisées de l'atlas `campaign_impostors_albedo.png` (tuiles de 160 m), sinon les cartes de feuilles disparaissent dans les mips.
- Bois lointains (`decor/far_woods.rs`) : anneau lointain Godot (grille 42 m, 840 m à 7 km, seuils `far_woods`, forêt réelle de la tuile d'horizon), cartes d'imposteurs par tuiles de 2 km.
- Houppiers : projettent des ombres.
- Vignes : maillage recentré (fondu `VisibilityRange` mesuré à l'origine).

- Panorama peint (`decor/panorama.rs` + `panorama.wesl`, port de `battle_panorama.gdshader`, `_choose_panorama`) : choix par `data/fx/horizon.json` (terrain, saison, mer), crête de `panoramas.json` et ligne d'horizon réelle de la tuile ; le cylindre de crêtes ne sert que de repli. Écarts : rayon 11,7 km (plan lointain Bevy 14 km), `light_tint` = 1 (teinte du soleil Godot non portée), silhouettes, mer lointaine et `battle_horizon_land` non portés.
- Lisière : les arbres hors du champ se posent sur la hauteur de l'anneau (`RealRelief` devient une ressource), plus sur le bord du champ.

- Armes et boucliers au sol (`units/arms.rs`) : ombres portées par un passage d'ombre optionnel par lot dans `units/instancing.rs` (`InstancedBatch::cast_shadows`, profondeur seule, comme `skinned.rs`) ; les armes tenues étaient déjà dans l'ombre des figurines. Hors de mes dossiers, modification minimale.
- SSAO : un préréglage de qualité avec `ssao` (moyenne et au-delà, `data/fx/render_quality.json`) coupe le MSAA de bataille et met SSAO + SMAA (`RenderQuality::ssao_battle`), sauf MSAA forcé par le joueur. Le matériau de vent (herbe, cultures, houppiers) n'a plus de prépassage : sa profondeur immobile trouait le sol (taches de ciel) dès que le SSAO activait le prépassage.
- Halo : la caméra HUD devient HDR (tonemapping coupé) pendant la bataille quand `bloom.enabled`, l'image n'est plus noire.

## Écarts / points ouverts
- Halo laissé désactivé dans les données : en HDR l'image vire au rose (ciel 131,133,142 → 154,129,134, même à intensité 0), cause non trouvée (chemin HDR de Bevy : étalonnage ou tonemapping en post-passe).
- Ombres des armes au sol : non vérifiées en jeu (aucune arme tombée dans les 300 premières images).
- La ligne d'arbres au loin reste plus claire et plus clairsemée que Godot (brouillard bleuté de Bevy sur les bois lointains) ; les normales des imposteurs Godot ne sont pas portées.

## Prochaine étape
- Armes / boucliers en ombre (opt-in par lot, `units/instancing.rs`), halo (caméra HUD HDR), SSAO (MSAA coupé + SMAA).
