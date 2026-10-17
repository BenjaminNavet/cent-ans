# Lot mapfx (post-playtest 17/10)

## État
- 1 Rochers : cause réelle = récepteurs d'ombre en cascade sur des triangles sous-pixel (bruit noir/blanc), albédo sombre (texture 0,07 linéaire, gain du catalogue non porté), grossissement x2,6. Corrigé : `NotShadowReceiver`, gain `color/albedo_mean` par affleurement (matériau copié par affleurement, plafonné x2,6), normales lissées, mip minimal pour les LOD 1-2, `far_scale` 2,6 -> 1,7.
- 1bis Mips : le chargeur glTF de Bevy ne produit aucun mip (textures 2048 lues texel par texel). `fx/model_mips.rs` génère les mips (2 images/image) de tout `StandardMaterial`.
- 2 Compteur `live.draught_beasts` : les bêtes d'attelage n'ont pas de `LifeItem` ; compté depuis les `Beast`.
- 3 Mât de Saint-Denis : c'est le moulin à vent (corps 0,71 u x 4,6 = 3,3 u de haut) ; `windmill_ratio` 1,0 -> 0,45.

## Reste
- 2 Charrette (bouillie) : à revérifier après les mips.
- 4, 5 : voir rapport.
