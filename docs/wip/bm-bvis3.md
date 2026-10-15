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

## Prochaine étape
- Panoramas peints de l'horizon (13 webp, `battle_panorama.gdshader`) : non portés.
- Armes / boucliers en ombre (opt-in par lot, `units/instancing.rs`), halo (caméra HUD HDR), SSAO (MSAA coupé + SMAA).
