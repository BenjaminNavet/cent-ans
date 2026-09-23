# État de l'application

Dernière mise à jour : 2026-09-23 (session 3, fin de M6).

## Où en est-on
- **M0 à M6 terminés** (M7 batailles en cours). Jeu jouable avec villes vivantes, dynasties, technologies, diplomatie et religion : guerres et paix négociées, alliances, vassaux, embargos, Papauté, Grand Schisme, hérésies.
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

- Technologies (M6) : 33 technologies (16 militaires, 17 civiles) datées et sourcées, points de recherche
  (base 5 + bâtiments de recherche + technologies + gouvernance du dirigeant ; France ≈ 20/tour), ordre
  `research` avec progression conservée en cas de changement, surcoût de 25 % pour les techs en avance
  de plus de 20 ans, effets appliqués au revenu, à la population et aux batailles, IA qui alterne les
  branches. Panneau Technologies (touche T) à deux onglets, jauge de recherche dans la barre.
  10 tests M6, smoke `_run_technologies`. Capture : `docs/img/godot-tech-tree.png`.

- Diplomatie et religion (M5) : attitude calculée avec raisons, casus belli (prétentions de 1337 : Édouard III sur la France, Philippe VI sur la Guyenne…), déclaration de guerre (réputation, parjure), appel aux armes, paix négociée (score de guerre, cessions, tribut, trêve de 5 ans), alliances, embargos (revenu), vassaux (tribut, loyauté, rébellion), mariages entre factions et prétentions dynastiques, union personnelle, propositions de l'IA au joueur, IA diplomatique minimale ; faveur pontificale, dons, médiation, excommunication, Grand Schisme 1378-1417 (obédiences historiques), Lollards (1381) et Hussites (1419). Une faction sans héritier voit une nouvelle maison (ou un élu) prendre le pouvoir. Panneau Diplomatie (P), modes de carte N/R. 18 tests M5. Sonde : `cargo run --release -p sim-campaign --example diplomacy_probe`.
- Correctif M4 : les effets du gouverneur s'appliquent désormais à la population et aux impôts.

- IA de campagne stratégique (M9, partie campagne) : crate `ai` (`ai::plan_turn`), utilisée par le pont pour toutes les factions IA. Objectifs par armée (défense des provinces menacées, sièges des provinces les plus précieuses, chevauchées des factions agressives, regroupement, retraite), fusion des armées, budget militaire (60 % du revenu en guerre, 30 % en paix), recrutement de la meilleure unité par livre, construction par rendement, impôts selon la guerre et l'ordre public, licenciement en cas de dette, gouverneurs, généraux, compétences, mariages ; diplomatie (M5) et recherche (M6) réutilisées. 9 tests. Sonde : `cargo run --release -p ai --example ai_probe` (100 tours, 0 % d'ordres refusés côté France). L'IA de bataille arrive avec M7.

- Sièges de campagne (M8, partie campagne) : vivres de la place (famine → capitulation), brèche ouverte par les engins de siège, ordre d'assaut (bouton dans le panneau d'armée avec estimation des chances), sortie de la garnison, IA qui donne l'assaut quand les chances dépassent 65 %. 7 tests M8.

- Objectifs et fin de partie (M10) : objectifs historiques par faction jouable dans `data/factions/*.json` (`victory`) — France : bouter les Anglais, tenir Paris et Reims, reprendre la Guyenne, soumettre la Bourgogne (1453) ; Angleterre : sacre à Reims, héritage Plantagenêt, 15 provinces du royaume, soumettre l'Écosse (1453) ; Bourgogne : indépendance, Pays-Bas, lien lorrain (1477). Victoire, défaite ou fin de campagne avec score ; panneau Objectifs (O ou Menu), écran de fin, objectifs sur les cartes du menu de départ. 5 tests.

## Limites connues
- `get_faction_summary` renvoie 0 pour projected_income/upkeep avant le premier tour (champs mis en cache en fin de tour) ; l'interface utilise `get_faction_economy` qui calcule à la volée.
- Les effets de bâtiments Garrison/RecruitCost/Supply sont exposés mais pas encore appliqués au gameplay ; le ciblage par classe des effets est ignoré.
- Les trésors s'accumulent vite (France ≈ 1,7 M, Empire ≈ 2,9 M livres en 25 ans avec l'IA stratégique) : l'équilibrage revenus/coûts est à faire en M10.
- L'IA minimale recrute une unité par tour et thésaurise ; l'IA complète est M9.
- La population ne varie pas encore (M3).
- Factions manquantes (Anjou-Provence, Grenade, Hollande-Hainaut, Brabant, Gueldre, Venise, Florence…) remplacées par la faction la plus proche, voir `docs/design/provinces-1337.md`.
- L'addon Blender MCP exige Blender ouvert en mode graphique ; le fallback headless est `tools/cent_ans_tools/blender.py`.

- L'IA ne propose pas encore de mariages ; les effets `Diplomacy`, `Intrigue`, `Loyalty` des traits/compétences restent sans effet (M9).
- M5 : l'IA diplomatique est volontairement prudente (peu de déclarations de guerre) ; la guerre de Cent Ans peut se conclure tôt par une paix blanche. Les noms des maisons générées viennent de la capitale ; le Portugal n'a pas de liste de prénoms dédiée.
- M6 : les effets de tech `army_upkeep`, `army_experience`, `recruit_cost`, `movement`, `production`,
  `siege_resistance`, `fortification_level`, `wealth`, `prestige` sont affichés mais pas encore appliqués ;
  `research_civil`/`research_military` (traits, compétences) sont inertes. Le surplus de points à
  l'achèvement est perdu. Le mock GDScript n'a pas de technologies.

## Commandes
- Build + tests : voir `CLAUDE.md`.
- Budget : `cd tools && uv run cent-ans budget show` (dépensé : 0,00 $ / 50 $).
