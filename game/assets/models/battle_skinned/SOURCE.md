# Figurines de bataille skinnées (lot V2)

- **Générées par** : `tools/blender_scripts/battle_skinned.py` (Blender 5.2, `blender -b --python`),
  avec `battle_skinned_figures.py` (recettes), `_equipment.py`, `_weapons.py`, `_poses.py`,
  `_cavalry.py`. Ne pas retoucher ces fichiers à la main : relancer le script.
- **Sources** (CC0 1.0, Quaternius, https://quaternius.com, via Poly Pizza) :
  `game/assets/third_party/characters/quaternius_modular_men/` (King, Adventurer, Farmer,
  Hooded Adventurer) et
  `game/assets/third_party/animals/quaternius_animated_animals/horse.glb`.
- **Modifications** : pièces modulaires assemblées et recolorées (codes matière du shader),
  équipement du XIVe siècle modelé par script (bassinet et camail, chapel de fer, heaume, écu,
  pavois, arc long, arbalète, piques, vouge, lance, caparaçon, selle), décimation en trois
  niveaux de détail, animations Quaternius rééchantillonnées à 24 i/s et complétées par des
  poses calculées (tir à l'arc long, arbalète, piques, cavaliers, morts, chutes).
  Lot UR1 : équipement régional et du XVe siècle (jaque, brigandine rivetée, salade et
  bavière, bassinet à visière, harnois blanc, rondache, targe, goedendag, coustille, hache
  d'armes, couleuvrine à main et poire à poudre, javeline, adarga, chanfrein et flançois)
  pour 13 figurines de plus (`infantry_3`-`8`, `archer_3`-`5`, `cavalry_3`-`6`).

## Figurines (manifeste : `style` d'animation et `noble`)
| Figurine | Unité | Style |
|---|---|---|
| infantry_0-2 | hommes d'armes, piquiers flamands, milice | sword, pike, militia |
| archer_0-2 | archers longs, arbalétriers, génois | bow, crossbow, crossbow |
| cavalry_0-2 | chevaliers (et chevaliers bretons), sergents, archers montés | lance, lance, horse_bow |
| infantry_3 | lanciers gallois | militia |
| infantry_4 | schiltron écossais | pike |
| infantry_5 | milice au goedendag | militia |
| infantry_6 | coutiliers | militia |
| infantry_7 | hommes d'armes des retenues (noble) | militia |
| infantry_8 | routiers | sword |
| archer_3 | francs-archers | bow |
| archer_4 | arbalétriers gascons | crossbow |
| archer_5 | couleuvriniers | crossbow |
| cavalry_3 | gendarmes d'ordonnance (noble) | lance |
| cavalry_4 | écorcheurs | lance |
| cavalry_5 | jinetes | lance |
| cavalry_6 | hobelars | lance |

Le type d'unité choisit sa figurine par le champ `figure` de `data/unit_types` ; les figurines
rigides de repli (B1/B4) n'ont que trois variantes par famille (`BattleSkinned.rigid_variant`).
- **Licence du dérivé** : CC0 1.0, comme les sources.

## Fichiers
- `manifest.json` : rigs (os, clips : ligne de départ, images, boucle) et figurines (fichiers,
  triangles par niveau, nombre de variantes).
- `<rig>.bones.bin` : `CAB1`, nombre d'os, nombre d'images (u32), puis zlib de 3 texels RGBA32F
  par os et par image (lignes d'une matrice 3×4 de skinning, espace Godot).
- `<figure>_lod<k>.mesh.bin` : `CAM1`, sommets, indices, taille brute (u32), puis zlib de :
  positions, normales, couleurs (rgb linéaire + code matière), UV, indices d'os ×4, poids ×4,
  masque de variante (float32), indices (u32). Lu par `game/scripts/battle/battle_skinned.gd`.
