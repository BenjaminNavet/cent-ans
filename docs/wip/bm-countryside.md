# bm/countryside

Etat : lot vague 4 termine pour le sous-ensemble ci-dessous (clippy -D warnings propre, 23 tests, dump verifie en campagne a Orleans, distance 40).

## Fait (Godot -> Rust, dossier client/src/campaign/countryside/)
- countryside_layer.gd -> config.rs, placement.rs, terrain_data.rs, layer.rs (scatter, paddock, village, road ; une scene glb par instance, plafond 1800 entites).
- river_crossings.gd + bridge_meshes.gd -> bridges.rs, bridge_meshes.rs (stone, wood, boats, ferry, ford, gate ; pont-porte non pose).
- war_scars.gd + war_scar_meshes.gd -> scars.rs, scar_meshes.rs (champs de bataille depuis `battle_history`, fosses de peste + charrette depuis `map_scenes`).
- decor_planner.gd + decor_layer.gd -> decor_plan.rs, decor.rs (sites, road, gate, crossing, coast, province, route ; glb du manifeste DN).
- map_prop_scale.gd : seuls les multiplicateurs maquette (countryside_size/range, folk_scale) sont utilises ; le reste concerne arbres/incendies (autres lots).

## Lot countryside2 (vague 5)
- water_props_layer.gd -> water_props.rs : moulins (berge, marée, bateau-moulin), jetées, grues, plages, arsenaux, chantiers, navires amarrés par bassin, épaves ; plan calculé sur un thread (`plan`, testé), scènes glb nourries autour du point visé (plafond 450), fondu `1 - strategic`, calage sur le relief affiché. Ajustement de modèle = `DnWaterModels._measure` (axe long sur X, `yaw_deg`, longueur, pied à 0). Niveau « water » = hauteur affichée du sol (pas de surface d'eau séparée).
- rock_outcrops.gd -> rocks.rs : catalogue `rock_outcrops.yaml`, semis par tuile sur thread (biome, altitude, pente, crête, lande, limite des arbres, groupes le long des courbes de niveau), scènes glb (plafond 320), taille réelle grossie jusqu'à `far_scale`, fondu `fade_start/end`. Non porté : roches de côte (`coast`), gain d'albédo/fondu pixel du shader, cisaillement vertical, plafond de triangles ; bruit de valeur au lieu du simplex.
- ground_clutter.gd, ponts-portes, engins de siège/croix de portes, fumées/volumes de repli du décor, shader countryside (teinte, fondu tramé) : non portés (instanciation GPU nécessaire pour le clutter ; plafond 1800 inchangé).

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
