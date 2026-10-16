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

## Lot bdecor5 (vague 8)
- `decor/camp_horse.rs` + `camp_horse.wesl` : chevaux au piquet animés (port de `camp_horse.gdshader` et de
  `AnimalMotion.build_camp_horses`) : respiration, poids d'un sabot arrière à l'autre, tête relevée / mâchonnement /
  hochement, queue. Réglages `camp_horse` de `data/fx/camp_horse_motion.json` (+ `enabled` de `animal_motion.json`) ;
  matériaux du glb conservés (extension de StandardMaterial), masque de queue sur les matières `Hair*`.
  `animal_motion_measured.json` ne sert qu'aux bêtes de la carte (campagne, déjà portées), pas aux chevaux du camp.
- Neige du kit : `site::snow_kit` = neige qui tombe ou sol enneigé (`_snowy` de battle_decor.gd), plus la saison hiver
  seule ; les variantes GA3 (`props.rs`) suivent la même règle (sous la neige, seuls chariot et puits en gardent,
  comme le lot « props » de Godot sans variante neige).
- `ga3_kit.gd` : déjà porté dans `props.rs` (scènes par instance au lieu de MultiMesh par tuile).
- Atlas FA7 : `decor/grass_atlas.rs` (catalogue `data/art/battle_grass.json`, cases tirées par bouquet, hauteur de la
  case, retournement) cuit dans les UV des chunks d'herbe ; groupe vert ou sans paquet seulement.
- Vergers : déjà dans `trees.rs` (`plan_orchards`) ; teinte de floraison (`orchard_blossom`) non portée (fichier showfix).
- Lumières des bâtiments : n'existent pas dans le décor de bataille de Godot (seules lueurs : feux de camp,
  `battle_smoke.gd`, et maisons en feu du siège, `siege_fire_fx.gd` = lot siège).

## Reste
- FA7 : blé / chaume / semis (cases de parcelles), groupes non verts (atlas recomposé), mips de l'atlas, herbe lointaine.
- Panorama peint, mer lointaine, imposteurs d'arbres (hors lot).
