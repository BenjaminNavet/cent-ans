# Lot mapfx (post-playtest 17/10)

## État
- 1 Rochers : cause réelle = récepteurs d'ombre en cascade sur des triangles sous-pixel (bruit noir/blanc), albédo sombre (texture 0,07 linéaire, gain du catalogue non porté), grossissement x2,6. Corrigé : `NotShadowReceiver`, gain `color/albedo_mean` par affleurement (matériau copié par affleurement, plafonné x2,6), normales lissées, mip minimal pour les LOD 1-2, `far_scale` 2,6 -> 1,7.
- 1bis Mips : le chargeur glTF de Bevy ne produit aucun mip (textures 2048 lues texel par texel). `fx/model_mips.rs` génère les mips (2 images/image) de tout `StandardMaterial`.
- 2 Compteur `live.draught_beasts` : les bêtes d'attelage n'ont pas de `LifeItem` ; compté depuis les `Beast`.
- 3 Mât de Saint-Denis : c'est le moulin à vent (corps 0,71 u x 4,6 = 3,3 u de haut) ; `windmill_ratio` 1,0 -> 0,45.

- 4 Oiseaux : échelle des espèces x0,35 (`data/map/map_birds.json`, une oie faisait 1,65 u, un homme 1,7 u). Modèles de la vie (charrettes, bêtes, faune, moulins) : plus de réception d'ombre (`life/models.rs::unshadow_model_meshes`).
- 5 Pavois : texture de planches (`wood_planks_diff.jpg`) teintée par camp, mips demandés via `MipQueue::request`.

## Non reproduits / non traités
- Chevaux de bataille (franges, plaques grises) : non reproduit en vue rapprochée à l'arrêt (cavalerie 3, d=14) ; à revoir en mêlée.
- Morts en croix, très clairs : non traité.
- Charrette : lisible de près (d=6) ; de loin (d=50) reste poreuse (modèles LOD2 800 triangles, `prop_model(.., 2)` toujours LOD2) -> choisir le LOD selon la distance.
- Chameaux : taille cohérente avec les figurines (folk_scale) ; l'écart vient des maquettes de monuments (non touché).
- Forêts d'hiver tramées, Pétra rose : non traités (vérifier le modèle `landmarks`).
- Brouillard : par conception la météo ne touche pas la couche de brume (`weather/atmosphere.rs`) ; non modifié.
- Budget de captures dépassé (~25) : le bruit des rochers a demandé de bisecter (mips, normales, ombres).
