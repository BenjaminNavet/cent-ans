# État de l'application

Dernière mise à jour : 2026-09-23 (session 3, fin de M4).

## Où en est-on
- **M0 à M4 terminés.** Jeu jouable avec villes vivantes et dynasties : personnages qui gagnent de l'expérience, apprennent des compétences, se marient, ont des enfants, meurent et héritent. Prochain jalon : M5 Diplomatie et religion.
- **M7 Batailles temps réel 3D terminé** (développé en parallèle de M5/M6) : quand une armée du joueur rencontre une armée ennemie, dialogue « Livrer bataille » / « Résolution automatique », bataille 3D avec pause, résultat rendu à la campagne.
- Design validé : `docs/design/2026-09-23-cent-ans-design.md`.

## Ce qui fonctionne
- `core/` : workspace Rust (data-model, sim-campaign, sim-battle, ai, godot-bridge). 10 tests, clippy propre.
- `game/` : projet Godot 4.7 chargeant la GDExtension ; scène principale avec date et bouton « Fin du tour ».
- Smoke test headless : `godot --headless --path game --script res://tests/smoke.gd` → 10 tours joués, « Automne 1339 ».
- `tools/` : projet Python (uv) avec ledger de budget, client images OpenRouter (contrôle du plafond 50 $), pilotage Blender headless, CLI `cent-ans`. 12 tests.
- `data/` : 9 schémas JSON (draft 2020-12) et 137 fichiers de données 1337 validés (15 factions, 39 personnages réels, 13 unités, 25 bâtiments, 22 technologies, 9 ressources, 6 religions, 8 provinces d'exemple).
- MCP : serveurs Godot (`@coding-solo/godot-mcp`) et Blender (`mcp-for-blender`, addon installé dans Blender 5.2) configurés dans `.mcp.json`.

- Carte réelle : `data/map/` (heightmap 4096² ETOPO 2022, Natural Earth), 132 provinces de 1337 avec polygones, voisins terrestres et maritimes (`cent-ans geo build`, ~20 s avec cache).
- Godot : scène `campaign_map.tscn` avec terrain par tuiles et LOD, frontières et teintes de faction en shader, mer animée, rivières, côtes, marqueurs de villes, caméra RTS, sélection de province, panneau parchemin. Captures : `docs/img/godot-real-map.png`.
- Rust : `GameDataStore` charge toutes les données typées et les expose à Godot.

- Simulation de campagne (`core/crates/sim-campaign`) : état complet, ordres validés, mouvement Dijkstra terre/mer, batailles auto, sièges, chevauchées, économie, ravitaillement, personnages (mort, succession), IA minimale, sauvegarde JSON, déterminisme testé sur 20 tours. 24 tests.
- Pont `CampaignSim` (GDExtension) : API complète spec M2 § 2, vérification headless `core/checks/campaign_sim_check.gd`.
- Interface Godot : menu de départ (3 factions), HUD parchemin (trésor, revenu, date, fin de tour, journal), panneaux armée et province, recrutement avec raisons, formation d'armée, marqueurs d'armées, ordre de déplacement au clic droit avec aperçu de chemin, sauvegarde/chargement. Captures : `docs/img/godot-campaign-hud.png`, `godot-campaign-province.png`, `godot-start-menu.png`.
- Équilibrage observé : France ≈ 24 000 livres/saison, armée royale de 8 unités ≈ 13 % du revenu.

- Villes et économie (M3) : population par classe avec quatre jauges dynamiques, construction de bâtiments avec prérequis et effets, capacité et surpopulation, biens par catégorie, taux d'imposition (Bas/Normal/Haut), révoltes (faction virtuelle `fac_rebels`), peste, famine. Onglet « Ville », panneau faction, mode carte mécontentement (touche M), marqueurs de construction. 15 tests M3. Équilibrage : France ≈ 22 500 livres/saison stable sur 5 ans (`cargo run -p sim-campaign --example income_probe`).

- Personnages et dynasties (M4) : XP (batailles, gouvernance), arbre de 30 compétences à trois branches, ~30 traits (acquis par événements, opposés exclusifs), généraux et gouverneurs qui modifient batailles et provinces, mariages, naissances (générées et historiques : Charles V naît en 1338 si Jean et Bonne sont mariés), succession par loi (salique, préférence masculine, cognatique, élective), régence des mineurs. Panneau Cour, fiche personnage avec arbre de compétences. 16 tests M4. Sonde : `cargo run -p sim-campaign --example dynasty_probe`.

- Batailles (M7) : `core/crates/sim-battle` simule au pas fixe de 0,1 s un champ procédural 1200 × 800 m (collines selon le terrain de la province, forêts, boue, rivière à deux gués), la météo de saison (pluie : arcs et arbalètes −40 %, brouillard : portée −30 %, neige), des régiments en ligne/colonne/schiltron/coin avec moral, fatigue, munitions, charge, flancs (+50 %) et dos (+100 %), piques contre cavalerie, pieux des archers, déroute et ralliement, aura et mort du général, et une IA minimale. Déterministe (même graine + mêmes ordres aux mêmes ticks = même bataille), 14 tests. `sim-campaign` met les batailles du joueur en attente (`pending_battles`, réglage `interactive_battles`), fournit `battle_setup`, applique `resolve_pending_battle` (pertes, moral, captures, général tombé, XP/traits M4, retraite) ou `auto_resolve_pending` ; les restes sont auto-résolus au tour suivant ; 8 tests M7. Godot : `scenes/battle/` (terrain maillé, arbres, soldats en MultiMesh par camp et famille, bannières, caméra RTS, sélection rectangle, ordres clic droit / glisser-droit, pause, vitesses ×1/×2/×4, HUD parchemin, écran de fin). 2 × 20 régiments de 120 soldats : 60 FPS (vsync), ~140 FPS sans vsync sur M4 Pro. Captures : `docs/img/godot-battle.png`, `docs/img/godot-battle-dialog.png`. Sonde : `cargo run -p sim-battle --example probe -- ai`.

## Limites connues
- `get_faction_summary` renvoie 0 pour projected_income/upkeep avant le premier tour (champs mis en cache en fin de tour) ; l'interface utilise `get_faction_economy` qui calcule à la volée.
- Les effets de bâtiments Garrison/RecruitCost/Supply sont exposés mais pas encore appliqués au gameplay ; le ciblage par classe des effets est ignoré.
- Le trésor du joueur s'accumule vite (≈ 300 000 en 5 ans sans dépense) : à rééquilibrer avec les coûts de M4-M6.
- Pas d'ordre d'assaut : les sièges se résolvent par durée uniquement.
- L'IA minimale recrute une unité par tour et thésaurise ; l'IA complète est M9.
- La population ne varie pas encore (M3).
- Factions manquantes (Anjou-Provence, Grenade, Hollande-Hainaut, Brabant, Gueldre, Venise, Florence…) remplacées par la faction la plus proche, voir `docs/design/provinces-1337.md`.
- L'addon Blender MCP exige Blender ouvert en mode graphique ; le fallback headless est `tools/cent_ans_tools/blender.py`.

- Les mariages ne sont pas inscrits au journal de la simulation (ordres immédiats) : l'interface affiche un message ; l'IA ne marie encore personne (M5).
- Les effets `Diplomacy`, `Intrigue`, `Loyalty` des traits/compétences sont stockés mais sans effet avant M5.
- Batailles (M7) : l'IA de bataille est minimale (avance en ligne, archers derrière, charge à 150 m) et les batailles d'IA décident vite (≈ 2-3 min simulées, déroute en cascade) ; pas de collisions entre régiments amis ; ligne de vue simplifiée (relief, forêts) ; les engins de siège tirent comme des archers lourds en bataille rangée ; les armées alliées présentes dans la province ne participent pas (seules les deux armées de la rencontre). Équilibrage à reprendre en M9/M10.

## Commandes
- Build + tests : voir `CLAUDE.md`.
- Budget : `cd tools && uv run cent-ans budget show` (dépensé : 0,00 $ / 50 $).
