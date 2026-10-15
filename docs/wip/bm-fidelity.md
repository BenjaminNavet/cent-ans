# bm-fidelity : écarts de rendu Bevy / Godot (section Bataille, lot bvis)

## Section Bataille : état après bvis
Corrigé : herbe (voir bm-bvis.md), fondu du sol aux bords des champs (bruit sur le splat), variation fine du sol, nuages, horizon moins sombre, ombres en cascade jusqu'à 450 m, texture des murailles du siège.

## Écarts restants, par impact visuel
1. **Soldats sans ombre portée** : le tirage instancié (`units/instancing.rs`, `skinned.rs`) ne passe pas dans la phase d'ombre ; les formations flottent. Piste : phase `Shadow` avec un pipeline vertex seul (même habillage GPU), ou ombres planes projetées (dither).
2. **Matières des bâtiments** (lot mats) : maisons, tentes, église, tours blanches ou grises unies.
3. **Sol** : l'albédo mipmappé perd tout détail au-delà de 3 m (vérifié par `textureSampleLevel`) ; le rendu repose sur le bruit procédural et l'herbe. Godot : sol peint plus contrasté, labours rayés, chemins. Piste : micro-texture de détail à fort contraste (échelle 0,5 m), labours en sillons orientés, normales qui survivent aux mips (plus forte intensité).
4. **Horizon** : pas de lisière d'arbres proche (anneau de bois à 520-900 m seulement), collines absentes sans relief réel ; ciel : nuages procéduraux mais pas de HDRI.
5. **Lumière** : pas de SSAO ni d'étalonnage (saturation, exposition) ; soleil sans diffusion atmosphérique.
6. **Soldats lointains** : imposteurs petits, mâts et fanions dominent ; livrées saturées. Fantassins vus de dos : grands boucliers bleus unis.
7. **Plaques de régiments** : nuée de cadres blancs (lot hud).
8. **Vignobles** (`site.rs`) : même défaut de fondu que l'herbe (maillage en coordonnées monde, `VisibilityRange` mesuré à l'origine) ; recentrer le maillage.
9. **Siège** : tours à cônes unis, créneaux absents, ville du kit sans matériaux.

## Leçon technique
Dans Bevy 0.20, le fondu `VisibilityRange` côté GPU (`get_visibility_range_dither_level`) mesure la distance à l'origine du maillage (translation de la transformation), quel que soit `use_aabb`. Tout maillage fusionné en coordonnées monde sous une transformation identité disparaît.
