# BR1 — Bâtiments réalistes (campagne et bataille)

Branche `br1-buildings`, worktree `../gp-br1`. ADR 0021. Session séparée (le joueur est absent
plusieurs heures, autonomie complète).

## État

- [x] 0. Squelette : ADR, wip, textures Poly Haven (`game/assets/textures/buildings/`)
- [x] 1. Kit Blender : `kit_geometry.py` (géométrie pure Python : murs percés à tableaux, pans
      de toit épais subdivisés, poutres orientées, patine par bruit, occlusion cuite),
      `building_kit.py` (12 recettes high/low : chaumière, longère, colombage à encorbellement,
      maison de ville à pignon sur rue, maison de pierre, grange, église, cathédrale gothique,
      maison forte, halle, puits, moulin ; ruines calcinées ; faîtage affaissé, faux-aplomb),
      `kit_export.py` (GLB + manifeste, aperçus Eevee, `preview-glb`)
- [x] 2. Godot : `building_materials.gd` (matériaux PBR partagés par nom, variante neige,
      matériau atlas `Building`), `building_kit.gd` (manifeste, choix par emprise, `Batch`
      MultiMesh, poignées pour cacher une maison)
- [x] 3. Bataille : `battle_village.gd` (maisons du kit selon type/emprise de la simulation),
      `battle_siege.gd` (îlots de deux rangées dos à dos de maisons de ville par disque,
      faubourgs ruraux, grande église ; `ruin_site(i)` pose la ruine du kit quand le feu a
      tout brûlé, appelé par `siege_fire_fx.gd`)
- [x] 4. Campagne : `kit_campaign.py` branché dans `models.py` / `settlements.py` (maisons,
      églises et cathédrale du kit en détail bas, villes fortifiées en rues bordées de rangées
      mitoyennes, pièces anciennes texturées : murailles en `Masonry`), matériau atlas unique
      (`building_atlas.gdshader`, `building_albedo_array.jpg`, couche = alpha de la couleur de
      sommet) : 3 surfaces par maquette
- [x] 5. Captures `docs/audit/captures/br1/` (avant/après siège, village, Paris, Rouen, Troyes)
- [x] 6. Fusion dans main (ff, 8a2198c9)

## Mesures (charge machine ≈ 85, FPS non significatifs ; appels et primitives fiables)

| Vue campagne | Avant (main 45f7ad5) | Après BR1 |
|---|---|---|
| Paris d = 25 | 746 appels / 7,64 M prim. | 257 appels / 7,94 M prim. |
| Paris d = 70 | 1 250 appels / 9,77 M prim. | 471 appels / 10,26 M prim. |

## Commandes

- Aperçus : `blender -b --python tools/blender_scripts/kit_export.py -- preview <préfixe> cottage:11 timber:31 townhouse:47! [--low]`
- Export bataille : `blender -b --python tools/blender_scripts/kit_export.py -- export game/assets/models/buildings`
- Campagne : `blender -b --python tools/blender_scripts/settlements.py -- game/assets/models/settlements`
  et `blender -b --python tools/blender_scripts/models.py -- game/assets/models castle city_cathedral town village cathedral`
- Captures : `godot --path game --resolution 1600x900 --script res://tests/br1_buildings_shot.gd -- --out=<dossier> --village --terrain=plains`

## Limites / suites

- La disposition des maisons de siège vient du cœur (`siege.rs`, 2 anneaux de disques de 9 m) :
  la ville reste aérée. Densifier (3e anneau, rues plus étroites) change le cheminement et la
  propagation du feu : à décider avec l'équilibre (A1-12 côté règles).
- Le Paris emblématique (L1) remplacera la maquette générique `castle.glb` de Paris.
- Pas encore de mobilier (étals, charrettes, tonneaux) dans les rues de siège.

## Prochaine étape

Polissage : mobilier de rue du siège, captures hiver ; densité de siège à décider côté règles.
