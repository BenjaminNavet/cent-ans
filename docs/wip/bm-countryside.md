# bm/countryside

Etat : lot vague 4 termine pour le sous-ensemble ci-dessous (clippy -D warnings propre, 23 tests, dump verifie en campagne a Orleans, distance 40).

## Fait (Godot -> Rust, dossier client/src/campaign/countryside/)
- countryside_layer.gd -> config.rs, placement.rs, terrain_data.rs, layer.rs (scatter, paddock, village, road ; une scene glb par instance, plafond 1800 entites).
- river_crossings.gd + bridge_meshes.gd -> bridges.rs, bridge_meshes.rs (stone, wood, boats, ferry, ford, gate ; pont-porte non pose).
- war_scars.gd + war_scar_meshes.gd -> scars.rs, scar_meshes.rs (champs de bataille depuis `battle_history`, fosses de peste + charrette depuis `map_scenes`).
- decor_planner.gd + decor_layer.gd -> decor_plan.rs, decor.rs (sites, road, gate, crossing, coast, province, route ; glb du manifeste DN).
- map_prop_scale.gd : seuls les multiplicateurs maquette (countryside_size/range, folk_scale) sont utilises ; le reste concerne arbres/incendies (autres lots).

## Reste (non porte)
- ground_clutter.gd : touffes d'herbe/broussailles instanciees (MultiMesh, shader `ground_clutter`, cartes de plantes par biome TX T3).
- rock_outcrops.gd : affleurements TRELLIS (`data/art/rock_outcrops.yaml`, `models/rocks/hb/`), semis par tuile sur pente/crete/biome.
- water_props_layer.gd : moulins a eau, jetees, grues, navires amarres, chantiers (`data/art/dn_water_models.json`).
- war_scars.gd : croix sur les portes des maisons 1:1 (plans de ville du lot towns), engins de siege pres du camp (marqueurs d'armee + `get_assault_odds`), provinces masquees des marqueurs d'armee.
- bridges : ponts-portes sous les colonies (`RiversRenderer.covers`), mode fin (`fine_anchors.json`), ponts glb generes (`DnWaterModels`), uniformes gue/bac de `river_water.gdshader`.
- decor : volumes procedurals de repli (tous les types ont un glb), panaches de fumee des types `smoke`, ombres par distance, fondu par transparence (ici reduction d'echelle).
- layer : instanciation GPU (une scene par instance), albedo/saison/vent du shader `countryside.gdshader` (teinte aleatoire, profils de saison par regle = presence seulement), fondu tramé des `fixed_size`.
- devastation : les scenes `devastation` sont mises en scene par la couche folk du lot life, pas ici.

## Ecarts connus
- Generateur aleatoire splitmix (pas PCG de Godot) : memes regles et densites, positions differentes.
- Les ponts sous maquette de colonie ne sont pas filtres (Godot les masque par `covers`).
