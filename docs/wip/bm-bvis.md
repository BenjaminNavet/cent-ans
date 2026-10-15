# bm-bvis : qualité du rendu de bataille (client Bevy)

Branche `bm/bvis`. Méthode : captures hors écran (`run_bg.sh --offscreen`), comparées à des captures Godot du même type de vue (`tx_battle_ground_shot.gd`).

## Fait
- Herbe invisible : le fondu `VisibilityRange` du GPU mesure la distance à l'ORIGINE du maillage ; les mailles d'herbe étaient en coordonnées monde sous une transformation identité, donc tout était à > 300 m de l'origine. Chaque tronçon est recentré (transformation = centre) ; marges corrigées (visible de 0 à `fade_near`, fondu jusqu'à `fade_far`). Faces arrière : `double_sided: false` (la normale inversée noircissait la moitié des touffes). Densité : `spacing_m` 2,4 -> 1,5 (data/fx/battle_decor_look.json).
- Sol : variation fine (touffes 1,3 m / 3,7 m), dérives chaudes / fraîches (61 m), `macro_contrast` 0,28 -> 0,45. Cause du sol « plat » : l'albédo mipmappé perd tout détail au-delà de 3 m (vérifié lod 0 à 4).
- Horizon : `skyline_haze.clear` 0,5 -> 0,82 (la crête était une bande sombre contre le ciel).
- Ciel : nuages procéduraux (`sky_clouds.wesl`, `sky_clouds.rs`), selon la météo.
- Dev : `CENT_ANS_BATTLE_CAM=<dist>[,dx,dz[,yaw_deg]]`, `CENT_ANS_BATTLE_MELEE=<dist>` (suit la 1re mêlée).

## Restes
Voir docs/wip/bm-fidelity.md § Bataille si présent.
