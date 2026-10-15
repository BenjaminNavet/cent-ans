# BM towns : villes détaillées au zoom proche (client Bevy)

Lot `client/src/campaign/towns/` (branche bm/towns).

## État
- Fait (pur, testé) : `data.rs` (towns_1340.json, règles de plan/murs, croissance), `kit.rs` (manifest du kit glTF, choix par proportions), `plan.rs` (port de `town_plan.gd` : rues drapées, îlots, parcelles, marché, paroisses, monuments, murailles, tours, portes, pont, sol), `growth.rs` (port de `town_growth.gd`), `profile.rs` (`town_render.tres`), `rng.rs`.
- Données : `data/art/town_look.json` + schéma (teintes, rubans, mobilier de place).

## Fait (vague 4)
- `layer.rs` : flux, plan, blocs / kit, rubans, murs, monuments, mobilier de place, croissance sim, fondu, `GroundHeight`, dump `campaign_towns`. Preuve : run_bg Abbeville (6 villes, 1743 maisons, 3167 blocs ; mode detail : 1792 modèles du kit, 0 erreur ; croissance forcée 3 quartiers + pierre : 252 maisons, 347 pièces).
- Pas de capture (règle commune) : aspect des blocs, teintes et soulèvement des rubans non vérifiés à l'oeil.

## Passe 2 (lot towns2)
- Correctif : les ancrages de `towns_1340.json` datent de l'avant carte composée ; `realign_to_settlements` les remet sur `settlements_px.json` (sans cela aucune ville 1:1 ne se construisait). Preuve : Amiens, caméra xz 1630,2383 : 3089 maisons, 3117 modèles du kit, 1 maquette enfoncée.
- `regions.rs` : styles régionaux des maisons (`building_regions.json`, part de colombage + variantes du Midi, exceptions par province), `Kit::pick_styled`, test sur les données réelles (Languedoc : variantes du Midi ; France nord : colombage > 70 %).
- `outbuildings.rs` : bâtiments hors les murs en style `maquette` (port partiel de `outbuilding_layer.gd`) : niveaux par bâtiments construits et ressources (`models_for`), famille `mill` seulement si construite en cours de partie, taille monde constante (fraction d'écran à `size_distance`), secteur par famille, mer évitée, portée 330, brouillard de guerre. Dump `campaign_outbuildings` (1003 modèles, 349 lieux).
- Markers : maquette enfoncée sous sa ville 1:1 (échelle y = 1 - poids, masquée à 0), épinglage capitale/sélection, écu de l'occupant (RJ-c), pose au plus bas entre le centre et la moyenne de l'emprise (`_pose_m`).

## Restes
- Villes emblématiques 1:1 (`landmark_plan.gd` 809 l., `landmark_monuments.gd` 566 l., `landmarks_v2/*.json`) : non portées ; les 8 lieux gardent leur maquette Meshy.
- `town_far_*` : remplacé par les maquettes (ADR 0158), rien à porter.
- Hors les murs : `_layout` (paliers sans recouvrement), sites rive/route/pente, signes de loin, faubourgs de loin, fondu d'opacité, chantier `worksite_1` (dessiné par markers), style `real` (taille tenue à l'écran).
- Styles régionaux : seul le choix des modèles est porté ; les matières régionales (`tx_building_pack.json`, shader `building_regional`) demandent un matériau WGSL.
- Arbres des jardins, pont, toits en prisme, ombres, plan sur fil de travail.

## Passe 3 (lot towns3, branche bm/towns3) — EN COURS
- Fait (pur, testé) : `landmark_v2.rs` (bibliothèque `landmarks_v2`, repères, dates), `landmark_plan.rs` (port de `landmark_plan.gd` : quartiers indexés, eaux, monuments, enceintes en anneaux, tours, portes, ponts + chapelle + maisons du pont, quais, places, rues réelles, parcelles importées de Paris, parcelles en lanières, sol), `landmark_monuments.rs` (port de `landmark_monuments.gd` : 10 gabarits, maillages à couleurs de sommet). Les 8 villes se planifient en 2-35 ms (Paris : 8871 maisons, 109 monuments).
- Fait (vague 5) : branchement dans `layer.rs` + `layer/landmark.rs` : flux (la ville la plus proche dans le rayon, une par image), clef = id de colonie (`set_paris`, lu par `TownsState::is_shown`), fondu commun, ancre = `settlements_px.json` (pas de projection EPSG:3035 sur la carte composée), maisons kit/blocs, maisons et chapelle du pont posées au niveau de l'eau (`TownGround::sample`), monuments à maillages couleurs de sommet, anneaux de murs (normales explicites, redessinés au changement de relief), tours, portes, ponts (tablier, piles, avant-becs, rampes). Preuve : run_bg Campagne, caméra xz 1651,2731 : Paris 8871 maisons, 109 monuments (12 248 triangles), 2 anneaux (5,3 km), 78 tours, 20 portes, 4 ponts (56 maisons), 0 erreur ; `CENT_ANS_TOWN_LOD=detail` : 8876 modèles du kit. Plan en 31 ms, pose 8 ms.
- Aucune capture exploitable : le rendu hors écran de run_bg (`--shot`) ne dessine pas le monde (écran noir avec l'interface).
- Fait (extras) : `shapes.rs` : bloc de maison à toit en prisme (murs enduits, toit de tuiles, sommets colorés) pour tous les blocs de maisons (villes ordinaires, croissance, villes emblématiques) ; arbres des jardins (`plan.trees`, tronc + houppier fusionnés en un seul maillage par ville, détail seulement, recalés au changement de relief) : Paris 2409 arbres.
- Restes : plan sur fil de travail (le plan de Paris prend 31-41 ms, sur le fil principal comme les villes ordinaires ; demande un instantané de hauteurs sûr entre fils) ; matériaux régionaux WGSL ; ombres ; `reground` fin du plan (les maisons et monuments se reposent par `TownGround`, mais les niveaux de ponts et la base des tours/portes du plan restent ceux du relief résident à la planification) ; vitraux/atlas de matières BR1 (couleurs de couche approchées dans `landmark_monuments::layer_rgb`) ; hauteur de pont recalculée quand le relief fin arrive.

## Passe 4 (lot townfar, branche bm/townfar)
- `far/builder.rs` : port de `town_far_builder.gd` (F1 nappe polaire + jupe, faubourgs, enceinte, tours, portes, monuments, clochers ; F2 polygone 16 côtés + flèche). Sol : grille polaire `1 + 2n` échantillonnée sur le relief résident (`Surface::height_m_at`) au lieu du `ground_m` de `towns_1340.json` (antérieur à la carte composée).
- `far/mod.rs` + `far/town_far.wesl` : port de `town_far_layer.gd` + `town_far_mask.gd` : tuiles F1 (128 u) / F2 (512 u), génération dans l'`AsyncComputeTaskPool`, `ExtendedMaterial<StandardMaterial>` à étage de sommets drapé (`relief.wesl`), masque Rgba8 64 × 64 (R : ville 1:1 construite → enfoncée ; G : suie). Dump `campaign_town_far`.
- Écart assumé (consigne « faire mieux ») : Godot ne dessinait ce lointain qu'en style `real`. Ici il remplit la bande entre les villes 1:1 diffusées et les maquettes : tant que la couche 1:1 est active (rig < `max_rig_distance`), toute ville de `towns_1340.json` (sauf villes emblématiques) est à l'échelle 1:1 et sa maquette s'enfonce (`TownsState::is_shown`) ; au-delà, le lointain s'enfonce avec le poids pendant que les maquettes remontent. `CENT_ANS_TOWN_FAR=0` le coupe.
- `soot.rs` : port de `town_soot.gd` (dévastation, siège, prise/sac, décroissance), config `building_models.json` → `soot` ; appliquée au lointain seulement. `CENT_ANS_TOWN_SOOT=set_x:0.8,...` force une suie.
- `level.rs` : `SettlementGrowth.level_of` / `has_castle` (niveau visuel, pur). Les maquettes CV1 et le château Kenney ne sont pas portés (remplacés par les maquettes, ADR 0158).
- Non portés, hors sujet ici : `town_render_profile.gd` (déjà `profile.rs`), `settlement_slot_bar.gd` (déjà dans panels/province), `settlement_cells.gd` (grille de dé-encombrement des noms : dossier markers), `building_materials.gd` (atlas PBR des matières du kit : demande un matériau WGSL dédié).
- Restes : suie sur les villes 1:1 et les bâtiments hors les murs ; lointain v2 des villes emblématiques (`build_v2_far`, besoin d'un découpage/triangulation de polygones) ; teinte « masse de toits » (`roofscape`) ; plaques de suie par fragment (ici par sommet) ; ombres du F1 ; facteurs de qualité.
- Preuve (run_bg, Amiens xz 1630,2383) : distance 32 → couche active, F1 12 tuiles visibles (42 villes, 11 778 triangles), F2 145 tuiles (1614 villes, 82 284 triangles), 1 ville 1:1 masquée, 391/392 maquettes enfoncées, suie forcée sur 2 villes ; distance 60 → 0 tuile visible, 55 maquettes visibles. Génération 6,1-6,7 s (fond), 901 + 146 tuiles, 600 k triangles.
- Écran noir de `--shot` : pas dans towns/. Sans `--offscreen`, l'image est entièrement noire (moyenne 0) ; avec `--offscreen`, l'interface est rendue mais le monde 3D est noir, y compris à la distance 2600 (vue stratégique, aucune ville) et avec `CENT_ANS_TOWN_FAR=0`. Piste : caméra/cible hors écran de la campagne (le même défaut a été corrigé en bataille dans b010c588c : caméra d'interface clear/msaa). À voir par le lot campvis/caméra.
- Aspect du lointain non vérifié à l'oeil (aucune capture exploitable).
