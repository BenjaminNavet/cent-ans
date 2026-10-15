# bm-bdecor4 : décor de bataille et siège, vague 5 (suite de bm-bdecor3 et bm-bsiege)

Branche `bm/bdecor4`.

## Fait
- `battle/decor/kit_site.rs` : maisons, moulins, église, accessoires du site et des camps, murs de cimetière
  (tronçons de 10 m + porche du côté de la route la plus proche), chevaux des lignes de chevaux en modèles
  du BuildingKit (`campaign::towns::kit`, lecture seule) ; `site.rs` ne garde en volumes que ce que le kit
  n'a pas (poteaux et corde des lignes de chevaux, haies, palissades, ponts, genre absent du kit).
- `battle/decor/grass.rs` : groupes de biomes (`battle_grass_groups.gd`) : cartes lame / touffe / touffe
  dense du groupe du biome (`data/art/tx_battle_grass_pack.json`), teinte corrigée par la luminance de la
  carte. Ajoute la fonctionnalité `webp` de Bevy (`client/Cargo.toml`).
- `battle/siege/kit_town.rs` : ville de siège en modèles du kit (rangées de maisons de ville, église, faubourgs,
  mobilier), ruines du kit quand la maison a brûlé (`update_ruins`).
- `battle/siege/animation.rs`, `engines.rs`, `crew.rs` : bascule du trébuchet (courbe mesurée), mangonneau,
  recul de la bombarde + mantelet, servants skinnés `crew_0/1` (lot de tir déduit du rechargement du cœur).

## Reste
- Animaux animés des lignes de chevaux (`AnimalMotion`), camp pillé (tentes abattues), variante neige du kit,
  vergers, lumières des bâtiments, panorama peint, mer lointaine, imposteurs d'arbres, atlas FA7.
- Servants : fondu entre deux gestes, porteurs de munitions (`haul`), roues du bélier et de la tour,
  points de lâcher des projectiles (`release`).
