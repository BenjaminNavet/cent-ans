# bm-portaudit : audit de parité Godot → Bevy (ce qui reste à porter)

État (16/10) : **audit terminé**, aucun code. Branche `bm/portaudit`.
Base auditée : `bm/merge8` (vague 8 fusionnée : setbattle, diplo6, fxsmoke, naval2, bhud6, uisound2, setcamp,
flowinteg, uileft2, uiimg2, access, econ2, council2) + diff de `bm/access2`. Les lots en cours (parch2, bdecor5,
towns4, lifeanim2, uiinfra3, gore, access2, aflags2, bunits6, weather5, perf3, dataassets2, showfix) sont exclus :
leurs manques sont marqués « en cours » et ne sont pas repris dans les lots proposés.

## Méthode
1. 462 scripts `game/scripts/**/*.gd` + 132 shaders `game/shaders/*` ; pour chacun, recherche du nom de fichier et
   du `class_name` dans `client/src` (Rust + WESL) et dans les notes `docs/wip/bm-*.md`.
   Résultat brut : 350 scripts cités dans le code, 32 cités seulement dans les notes, 80 jamais cités.
2. Scripts jamais cités dans le code (50 `.gd`) : vérifiés un par un par fonction (clés de données, appels du cœur,
   textes d'UI) ; beaucoup sont portés sous un autre nom (onglets de diplomatie, aide F1, offre de missions…).
3. Scripts cités : un nom cité n'est pas un portage complet. Relevé des commentaires « Not ported » du client (86) et
   des sections « Reste » des ~90 notes bm-*.md, chaque reste revérifié par grep (les restes déjà faits par un lot
   ultérieur sont écartés ; voir § Commentaires périmés). Contrôle des citations purement négatives (6 cas).
4. Shaders : appariement par fonction (uniformes, effets) avec les `.wesl` / matériaux Bevy. Les 132 sont utilisés en
   jeu : 61 portés, 45 partiels, 17 absents, 9 sans objet.

## Bilan par impact joueur

| Catégorie | Nombre | Commentaire |
|---|---|---|
| Bloquant | **0** | Aucun écran ni boucle de jeu manquant : campagne, bataille, sièges, naval, menus, sauvegardes, codex existent. |
| Visible | **27** | Différence nette en partie normale (§ Manques visibles). |
| Mineur | **≈ 60** | Finitions, cas rares, petits effets (§ Manques mineurs). |
| Outillage / debug / plomberie (ignoré) | **26 scripts** | Fabriques Godot, autoloads, caches, sondes, garde-fou headless. |
| En cours dans un lot actif | ≈ 25 | Non repris ici (§ En cours). |

## Scripts jamais cités dans le code : verdicts

| Script Godot | Verdict | Bevy / remarque | Impact |
|---|---|---|---|
| battle/battle_prologue.gd (423 l.) | **absent** | didacticiel de bataille (`data/tutorial/battle_prologue.json`) ; le « Prologue » du menu Bevy est celui de la campagne | visible |
| ui/battle_prologue_invite.gd | absent | invite Oui / Non / Ne plus demander | mineur |
| battle/landmark_backdrop.gd | **absent** | toile de fond des sièges des 7 villes emblématiques (Seine, Cité, Notre-Dame…) | visible |
| battle/deployment_zone.gd | partiel | `battle/orders/deploy.rs` : contour seul, sans halo ni hachures | mineur |
| map/army_memory.gd | **absent** | armées ennemies perdues de vue (fantômes) ; `hover_text` prévoit `seen_ago`, jamais rempli | visible |
| map/feudal_map_lens.gd | **absent** | mode de carte Féodalité (+ 4 autres modes, lot mapmodes) | visible |
| map/fine_geo_layer.gd, fine_geo_store.gd, fine_ribbon_job.gd | **absent** | fleuves fins et routes drapées de près (`rivers_fine.json`, `fine_anchors.json`, tuiles E2) | visible |
| map/ground_cards.gd | absent | cartes de plantes TX T3 (`tx_ground_cards_array.webp`) ; `clutter.rs` le note | mineur |
| map/province_mates_overlay.gd | absent | colonies de la province en pastilles reliées à la cité (CO-B) | mineur |
| ui/menu_backdrop_3d.gd | absent (écart assumé) | menu en carte 2D illustrée au lieu de Paris 3D vivant | visible (décision joueur) |
| ui/faction_map_picker.gd (cité négativement) | absent | carte cliquable + filtres royaume/rang au choix de faction | visible |
| battle/battle_formation_outline.gd (cité négativement) | partiel | contours en gizmos (`orders/overlay.rs`) ; pas de décalque drapé ni d'états (survol, ennemi ciblé pulsé, mêlée) | visible |
| map/field_layer.gd | sans objet | désactivé par les données (`art/dn_fields.json` `enabled: false`) | — |
| map/dn_water_birds.gd | porté | `campaign/life/birds.rs` (`dn_model`) | — |
| map/help_controller.gd, legend_sample.gd | porté | `campaign/hud/help.rs` (aide F1 + légende) | — |
| map/mission_offer_controller.gd | porté | `campaign/hud/missions.rs` | — |
| map/panel_widgets.gd | porté | `campaign/panels/province`, `army` | — |
| map/heraldry_atlas.gd | porté | écus des noms de lieu (`markers`, LabelShield) | — |
| naval/naval_campaign.gd | porté | `flow/naval.rs`, `armies/sea.rs` | — |
| ui/diplomacy/* (7 fichiers) | porté | `campaign/panels/diplomacy` (négociation, guerre, cour, offres, liste) | — |
| ui/economy/economy_currency_tab.gd | porté | `campaign/panels/economy/view.rs` (Monnaie) | — |
| ui/user_dir_migration.gd | sans objet | même dossier `Cent Ans` ; **reste à la bascule** : `settings.cfg` Godot → `settings.json` Bevy non migré | mineur |
| Plomberie (26) : battle_kinds, multimesh_kit, particle_kit, battle_siege_batcher, expected_errors, headless_watchdog, perf_probe, frame_budget, tile_job_pool, settlement_cells, map_data_loader, map_paths, map_scale (`town_scale` = 1), sim_facade, sim_facade_ref, province_snapshot, province_section, rich_label, cmd_args, data_file, json_lookup, redraw_on_demand, game_catalog, ga3_vegetation, royal_acts_section (dans court) | ignoré | instanciation automatique Bevy, `game-api`, `paths.rs`, perf3 | outillage |

## Manques visibles (27)

| # | Manque | Godot de référence | Dossier Bevy | Lot |
|---|---|---|---|---|
| V1 | Modes de carte Religion, Loyauté, Ravitaillement, Revendications, Féodalité (6 modes sur 11) | map_mode_controller.gd, feudal_map_lens.gd | campaign/provinces/modes.rs | mapmodes |
| V2 | Mémoire des armées perdues de vue | army_memory.gd, map_hover_text.gd | campaign/armies | armies6 |
| V3 | Fleuves et routes fins de près (shaders `river_fine`, `road_fine`) | fine_geo_layer/store/ribbon_job.gd | campaign/water | finegeo |
| V4 | Eau douce : étangs, torrents, cascades, salines, gués | freshwater_layer.gd, freshwater_surface.gdshader | campaign/water | campwater2 |
| V5 | Couloirs maritimes à l'encre | sea_lane_layer.gd, sea_lane.gdshader | campaign/water | campwater2 |
| V6 | Bande côtière du terrain (plages, falaises) et géologie de côte | coast_band/coast_common.gdshaderinc | campaign/terrain | coastterr |
| V7 | Contraste du terrain ≈ moitié de Godot (post 1,06 / 1,08) | terrain.gdshader | campaign/terrain | coastterr |
| V8 | Forêt dense selon le zoom | forest_detail*.gd | campaign/vegetation | campveg5 |
| V9 | Vraies ombres portées des arbres (ellipses factices) | foliage*.gdshader | campaign/vegetation | campveg5 |
| V10 | Rivières de bataille en vraie eau (aujourd'hui sol teinté) | battle_water.gdshader, battle_terrain.gd | battle/scene | bwater |
| V11 | Mer de bataille (houle, Fresnel, écume) | battle_sea.gdshader, battle_foam.gdshader | battle/scene | bwater |
| V12 | Pluie et neige en bataille, brouillard volumétrique | battle_atmosphere.gd | battle/decor/atmosphere.rs | batmo |
| V13 | Ciel visible HDRI (dôme dégradé aujourd'hui), disque solaire | hdri_sky.gdshader, battle_sky.gdshader | battle/scene/lighting.rs, decor | batmo |
| V14 | Sol de bataille : normales, micro-grain, parcelles EP6, herbe couchée | battle_ground.gdshader, battle_common.gdshaderinc | battle/decor/ground.rs | bground |
| V15 | Arbres de bataille : squelette, 3 LOD, écorces, rameaux d'hiver | battle_trees.gd, battle_tree_*.gdshader | battle/decor/trees.rs | bground |
| V16 | Poussière des régiments, traînées de projectiles | battle_effects.gd, battle_projectile_trail.gdshader | battle/effects (nouveau) | bfx |
| V17 | Projectiles de siège en vol (pierres, carreaux) | siege_assault_fx.gd | battle/siege/fx.rs | bfx |
| V18 | Flèches enflammées, pieux et pavois plantés | battle_volleys.gd | battle/units/volleys.rs | bfx (après bunits6) |
| V19 | Lueur nocturne des feux, braises | battle_smoke.gd, fire_ember.gdshader | fx/, battle/effects | bfx |
| V20 | Contour de formation drapé avec états | battle_formation_outline.gd | battle/orders/overlay.rs | border2 |
| V21 | Toile de fond des sièges des villes emblématiques | landmark_backdrop.gd, landmark.gdshader | battle/siege | siegeland |
| V22 | Avant-bataille : minicarte du site, météo prévue, médaillons des généraux | pre_battle_dialog.gd | flow/dialog.rs | predlg |
| V23 | Choix de faction : carte cliquable, filtres, portraits, chiffres clés | faction_map_picker.gd | menu/faction_select.rs | menu5 |
| V24 | Didacticiel de bataille (prologue) | battle_prologue.gd, battle_prologue_invite.gd | menu/, battle/tutorial (nouveau) | bprologue |
| V25 | Panneau Tech : recherche texte, retrait de la file (`DequeueResearch`) | tech_tree_view.gd, tech_panel.gd | campaign/panels/tech | campui6 |
| V26 | Codex : bulles épinglées (clic droit, T), bulles `ib:` d'entités, bulle de décor (ADR 0219) | codex_bubbles.gd | codex/ | codex3 |
| V27 | Menu principal 3D (Paris vivant, `menu_ground`, `menu_grass`) | menu_backdrop_3d.gd | menu/ | menu3d (**décision joueur** : écart assumé à ce jour) |

## Manques mineurs (≈ 60, regroupés)

- **Campagne, eau** : lit creusé / `water_sink` / `minor_max_distance` (river_bed, river_water) ; reflets `water_glint` ;
  glaciers et névés (`freshwater_ground`) ; ponts-portes sous les places et ponts glb fins (bridges.rs) ;
  ombres de nuages sur la mer ; textures de ressac (`coast_look`) — lots campwater2 / finegeo / coastterr.
- **Campagne, décor** : cartes de plantes TX T3 ; rochers de côte et `ground_rocks` proches ; vent / fondu du clutter ;
  cicatrices (engins près du camp, croix des portes) ; panaches `smoke` du décor ; arbres d'hiver ajourés
  (`foliage_*_winter`) ; voile de bord de carte sur arbres (`campaign_edge`) ; météo par province (`weather_field_bake`) ;
  masse de toits lointaine (`roofscape`) et matières régionales TX T4 (`building_regional` ; vérifier towns4) — lot campveg5.
- **Campagne, armées** : liste des armées (sceaux, jauges, pictogrammes, Tab) ; voiles de cogue aux couleurs et armes
  de la faction (`campaign_sail`) ; masque de la bulle d'atteinte (`reachable_bubble`) — lot armies6.
- **Campagne, UI** : brouillard et relief ombré de la minicarte ; colonies de la province (`province_mates_overlay`) ;
  clic sur une case vide → chantier ; recadrage des vignettes du rapport de saison ; lien Codex du didacticiel ;
  choix campagne courte / longue ; événements féodaux de fin de tour + guide ; chronique (sceaux de cire, liens Codex,
  clic → caméra) ; cour (NextHint, `portrait_marks`, zoom/glisser de l'arbre) ; diplomatie (chancellerie, don à
  l'Église, arbitrage féodal, montants libres) ; illustrations des rencontres (asset absent) — lots campui6 / campui7.
- **Bataille** : feux de camp animés (`battle_camp_fire`), flamme/eau/mer animées du décor (`features.rs`), fumées
  d'horizon, `battle_horizon_land` par fragment ; ombres des imposteurs ; zone de déploiement (halo, hachures) ;
  trajets d'ordres par segment ; rejeu (+/−) ; infobulles des badges de mode ; roues/balancement du bélier et de la
  tour, fondu des gestes des servants ; cinématique de prise de point ; cimetières, maisons du site en BuildingKit ;
  étendard lié à l'os du porte-étendard ; frémissement / transmission des feuilles ; fondu doux de la fumée.
- **Menus / flux** : bouton « Aide (F1) » et infobulle de difficulté de la pause ; invite du didacticiel ; type de lieu
  « siège » en bataille personnalisée (abandonné par le cœur → sans objet probable).
- **Audio** : limiteur / compresseurs de bus ; `ui_order` dans diplomatie et autres panneaux ; durée réelle des voix ;
  `court_open` perdu sur Échap ; instances max en muet — lot audio4.
- **Codex** : icônes d'onglets, `format_auto` dans les textes de chronique — lot codex3.
- **Réglages** : qualité Basse non branchée aux bandes de frontière ; MetalFX / SSAO-SSIL / packs (ligne sans effet) ;
  migration `settings.cfg` → `settings.json` à la bascule — lot setwire2.

## En cours dans un lot actif (non repris)
Parchemin (frontières épaisses, hachures d'occupation, noms de région, météo, jetons fantômes, légende) : parch2.
Chevaux de camp (`camp_horse`), kit neige, vergers, lumières, panorama peint, mer lointaine, imposteurs d'arbres,
atlas FA7 : bdecor5. Villes 1:1, dépendances, TX T4 : towns4. Folk animé, fondu tramé du folk, `folk_prop`, battement
d'ailes : lifeanim2. Sang, cadavres, membres, armes tombées (`battle_gore`) : gore. Armoiries des imposteurs, atlas
cuit, ombres des armes : bunits6. Cumulus (mer, `wet`), aurore, brume par province : weather5. Bannière en mer,
pastille de posture : aflags2. Daltonisme, contraste élevé : access2. FadeToState, ArtPlates, écran de chargement
plein écran, `build_companion`, poignée : uiinfra3. Rivières enfouies, ciel blanc, arbres proches pâles, cônes des
tours : showfix.

## Lots proposés (prêts à lancer)

Taille : S ≈ ½ j agent, M ≈ 1 j, L ≈ 2 j. « Après X » = attendre la fusion de X (dossier partagé).

| Lot | Dossier cible | Godot de référence | Contenu | Taille | Dépendances | Modèle |
|---|---|---|---|---|---|---|
| **mapmodes** | campaign/provinces (modes.rs, fill.wesl) + `game-api` (vue `lens`) | map_mode_controller.gd, feudal_map_lens.gd | 5 modes manquants (religion Avignon/Rome/hérésies, loyauté, ravitaillement, revendications, féodalité : fond du souverain, hachures du tenant, écu parti), boutons et infobulles | M | après parch2 (hachures partagées dans border.wesl) | Sonnet |
| **armies6** | campaign/armies | army_memory.gd, unit_roster_controller.gd, campaign_sail.gdshader, reachable_bubble.gdshader | fantômes des armées perdues de vue (`stance_cues.json` `fog_memory_turns`), liste des armées (sceaux, jauges, Tab), voiles héraldiques, masque de bulle | M | après aflags2 | Sonnet |
| **finegeo** | campaign/water (+ countryside/bridges.rs) | fine_geo_layer.gd, fine_geo_store.gd, fine_ribbon_job.gd, river_fine/road_fine.gdshader, river_crossings.gd | fleuves fins et routes drapées sous le palier comté, ancrages fins, ponts-portes | L | après showfix (water/ribbon.rs, reseat.rs) | Opus |
| **campwater2** | campaign/water | freshwater_layer.gd, freshwater_surface.gdshader, sea_lane_layer.gd, sea_lane.gdshader, water_glint.gdshaderinc, river_bed.gdshaderinc | étangs/torrents/cascades/salines/gués, couloirs maritimes, reflets, lit creusé + `minor_max_distance` | M | après showfix ; fichiers neufs si en parallèle de finegeo | Sonnet |
| **coastterr** | campaign/terrain | coast_band/coast_common.gdshaderinc, freshwater_ground.gdshaderinc, terrain.gdshader (post) | bande côtière, géologie de côte, glaciers et névés, contraste/saturation post-tonemap 1,06/1,08 | M | après showfix (terrain.wesl, carve) et weather5 | Opus |
| **campveg5** | campaign/vegetation + campaign/countryside | forest_detail*.gd, ground_cards.gd, rock_outcrops.gd, war_scars.gd, ground_rocks/foliage_*_winter.gdshader | forêt dense par zoom, ombres portées réelles des arbres instanciés, cartes de plantes TX T3, rochers de côte, cicatrices restantes, panaches, hiver ajouré | L | après lifeanim2 et perf3 | Opus |
| **bwater** | battle/scene (nouveau `water.rs`) | battle_water.gdshader, battle_sea.gdshader, battle_foam.gdshader, battle_terrain.gd | rivières de bataille en maillage d'eau, mer avec houle/Fresnel/écume, écume aux piles de pont | M | après showfix et bdecor5 (mer lointaine) | Sonnet |
| **batmo** | battle/decor/atmosphere.rs, battle/scene/lighting.rs | battle_atmosphere.gd, hdri_sky.gdshader, battle_sky.gdshader, battle_horizon_smoke.gdshader | ciel HDRI visible + disque solaire, pluie/neige, brouillard volumétrique, LUT lift/gain, fumées d'horizon | M | après showfix (ciel blanc) et bdecor5 | Opus |
| **bground** | battle/decor (ground.rs, trees.rs, grass.rs) | battle_ground_textures.gd, battle_trees.gd, battle_ground/common/tree_*.gdshader | normales du sol, micro-grain, parcelles EP6, herbe couchée ; arbres à squelette, 3 LOD, écorces, rameaux d'hiver | L | après bdecor5 | Opus |
| **bfx** | battle/effects (nouveau) + battle/siege/fx.rs + fx/ | battle_effects.gd, siege_assault_fx.gd, battle_projectile_trail.gdshader, fire_ember.gdshader, battle_smoke.gd | poussière des régiments, traînées, projectiles de siège en vol, braises, lueur nocturne des feux ; flèches enflammées / pieux / pavois dans volleys.rs | M | volleys.rs après bunits6 | Sonnet |
| **border2** (ordres de bataille) | battle/orders, battle/hud | battle_formation_outline.gd, deployment_zone.gd/.gdshader, battle_replay_controller.gd, battle_ability_icons.gd | contour drapé à états, zone de déploiement (halo, hachures), trajets par segment, rejeu +/−, infobulles des badges | M | aucune | Sonnet |
| **siegeland** | battle/siege | landmark_backdrop.gd, battle_capture_stage.gd, siege_engines_fx.gd, siege_crew_fx.gd | toile de fond des 7 villes emblématiques, cinématique de prise de point, roues/balancement des engins, fondu des gestes | M | après towns4 (modèles des villes) | Sonnet |
| **predlg** | flow/dialog.rs | pre_battle_dialog.gd | minicarte du site, météo prévue (prévisualisation `BattleSim`), médaillons des généraux | S | après uiinfra3 (ArtPlates) | Sonnet |
| **menu5** | menu/ | faction_map_picker.gd, pause_menu.gd, battle_prologue_invite.gd | carte cliquable des factions + filtres + portraits, pause « Aide (F1) » + infobulle de difficulté, invite du didacticiel | M | aucune | Sonnet |
| **bprologue** | battle/tutorial (nouveau) | battle_prologue.gd, data/tutorial/battle_prologue.json | escarmouche guidée en étapes, parchemin du didacticiel, entrée « Didacticiel de bataille » | M | après menu5 (invite) | Sonnet |
| **campui6** | campaign/panels/{tech,province}, campaign/hud | tech_tree_view.gd, tech_panel.gd, province_mates_overlay.gd, campaign_minimap.gd, season_report.gd, tutorial.gd | recherche Tech + retrait de file (`game-api::tech`), colonies de la province, case vide → chantier, minicarte (brouillard, relief), vignettes, lien Codex | M | aucune | Sonnet |
| **campui7** | campaign/panels/{diplomacy,feudal,chronicle,court} | diplomacy ui, feudal_controller.gd, chronicle_window.gd, court_panel.gd, portrait_marks.gdshader | chancellerie, don à l'Église, arbitrage, montants libres ; événements féodaux + guide ; sceaux de cire, liens Codex ; NextHint, `portrait_marks`, zoom de l'arbre ; `ui_order` des panneaux | M | aucune | Sonnet |
| **codex3** | codex/ | codex_bubbles.gd, encyclopedia.gd | bulles épinglées, bulles `ib:`, bulle de décor (picking 3D), icônes d'onglets, `format_auto` | M | après uiinfra3 (place_beside) | Sonnet |
| **audio4** | audio/ | audio_director.gd, disposition des bus | limiteur/compresseurs, durée réelle des voix, `court_open` sur Échap, instances en muet | S | aucune | Sonnet |
| **setwire2** | settings/ (+ campaign/provinces pour la qualité Basse) | settings.gd, faction_borders.gdshaderinc | qualité Basse → bandes de frontière, lignes sans effet masquées ou branchées, migration `settings.cfg` → `settings.json` | S | après access2, mapmodes | Sonnet |
| *menu3d* (optionnel) | menu/backdrop3d (nouveau) | menu_backdrop_3d.gd, menu_ground/menu_grass.gdshader | Paris au crépuscule, plans de caméra, ost en figurines | M | **décision du joueur** (écart assumé aujourd'hui) | Opus |

Ordre conseillé : sans dépendance, tout de suite : border2, menu5, campui6, campui7, audio4. Après fusion des lots en
cours : mapmodes, armies6, predlg, codex3, bfx, siegeland, setwire2. Après showfix : finegeo, coastterr, bwater,
batmo. Enfin : bground, campveg5, campwater2, bprologue.

## Commentaires « Not ported » périmés (à nettoyer au passage)
`campaign/hud/mod.rs:10` (minicarte faite), `campaign/panels/faction/mod.rs:27` (rançons et routes faites),
`codex/rules.rs:6` (onglets faits), `audio/advisor.rs:12`, `audio/voice.rs:4` (discours dans speech.rs),
`campaign/armies/figures.rs` (bivouac), `campaign/panels/tech` et `panels/diplomacy/mod.rs:17` (le champ texte existe),
`campaign/terrain/mod.rs` (saisons, parcelles, frontières faites dans terrain.wesl), `campaign/water/mod.rs` (textures
de détail faites dans sea.wesl).

## Points ouverts
- Le décompte par nom ne prouve pas la fidélité des 350 scripts cités : les écarts fins de rendu ne se verront qu'aux
  recaptures des 8 scènes (`tools/compare_views.sh`) prévues après les lots visuels.
- menu3d : écart assumé par `menu/main_menu.rs` ; à présenter au joueur.
- mapmodes et campui6 demandent des vues `game-api` (lentilles religion/revendications/loyauté, file de recherche) :
  le cœur expose déjà ces données au pont Godot (`get_map_lens`, `get_province_religion`, file de recherche).
