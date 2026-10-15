# bm-bdecor5 : décor de bataille, vague 6

Branche `bm/bdecor5`. Suite de `bm-bdecor4.md`.

## Fait
- `battle/decor/camp_loot.rs` : camp pillé (`BattleDecor._loot`). L'état des camps du cœur (`camp_state`) est
  relu chaque seconde ; au premier pillage, les tentes et pavillons du camp (`CampTent`, posés par `site.rs`
  d'après `kit_site::KitPlan::camp_tents`) s'abattent (bascule 0,35 rad, hauteur ×0,28) et jusqu'à 3 colonnes
  de fumée noire (`SmokeSources`, `column`) montent sur les feux. Dump `battle_decor.camp_loot` et `site.camp_tents`.
- Déjà couverts avant ce lot (grep) : BuildingKit (kit_site), groupes d'herbe, fumée/feux (`features.rs`).
  Le lot veg4 (`campaign::vegetation::textures`) est absent de la base : texture d'écorce/feuilles des arbres non traitée.

## Reste
- Animaux animés des lignes de chevaux (`AnimalMotion`), variante neige du kit, vergers, lumières des bâtiments,
  panorama peint, mer lointaine, imposteurs d'arbres, atlas FA7, packs d'écorce/feuilles (veg4).
