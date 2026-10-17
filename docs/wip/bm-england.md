# bm-england : Angleterre / Guyenne brunes, moulins noirs

Causes (preuves par échantillonnage du colormap, captures d=200 Londres, d=400 Guyenne) :

1. **Brun = voile du brouillard de guerre**, pas le colormap. Le colormap (BC1, moyenne sur ±100 km) donne Londres (117,132,54), Rouen (121,135,55), Paris (135,134,59), Bordeaux (101,126,55) : aussi verts que la Normandie. La coupure nette à la frontière anglaise (Guyenne) est la limite de vision du joueur : le sol hors de vue passe en camaïeu sépia (`fog_of_war` de `data/ui/campaign_map.json`, shader `fog_of_war_veil`). Réglage : `desaturation` 0,5 -> 0,2, `dim` 0,8 -> 0,95, `veil_strength` 0,1 -> 0,05 (l'inconnu reste plus terne et brumeux, il garde son vert).
2. **Biome** : le sud-est de l'Angleterre était « continental » (openfield labouré) car la mer du Nord au sud de 53,3 N n'était pas « atlantique ». Règle `oceanic_land_boxes` (biomes.yaml + schéma + `classify_koppen`) ; le `biomes.png` du dépôt n'est pas recuit en entier (un bake complet diffère sur 3,4 M pixels) : `cent-ans geo biomes-island` corrige en place 295 230 pixels (continental -> océanique, boîte de Bretagne seule), puis `geo biome-blend`. Londres et Cantorbéry deviennent océaniques (bocage, prairies) ; Paris et Rouen inchangés.
3. **Moulins noirs** : modèles `props/windmill_post`, `buildings/village_windmill_post_field` (couche countryside, ~100 dans la vue) : textures sombres (moyenne sRGB 66/62/55) sous un éclairage Bevy sans ambiant (80 cd/m2 par défaut contre ~380 voulus). Corrections : `GlobalAmbientLight` de la campagne calé sur l'ambiant du shader terrain (`lighting::scene_ambient`) et `albedo_gain` 2,8 (champ de données, comme Godot `countryside.gdshader`) appliqué aux matériaux des prototypes (`proto.rs::set_gain`).

Colormap non recuit (non en cause). Dump : `campaign_life.scene_ambient_cd` = 377 ; `sites.mills` 282 (moulins des sites de vie), countryside `windmill_*` 100 dans la vue.

Restes : décor `countryside_decor` (moulins à marée, polder) en scènes glb non lifts (ambiant seul) ; le voile de brouillard reste plus terne que le sol connu (voulu).
