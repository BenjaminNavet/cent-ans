# État de l'application

Dernière mise à jour : 2026-09-23 (session 3, fin de M10).

## Où en est-on
- **M0 à M10 terminés : la v1 est complète.** Campagne 1337-1453 jouable de bout en bout avec villes vivantes, dynasties, technologies, diplomatie et religion, chronique historique, batailles et sièges 3D temps réel avec IA tactique, objectifs historiques et écran de fin, sons, musiques, modèles 3D et écus ; application macOS autonome (`tools/export_macos.sh`). Reste : 49 portraits à générer (≈ 2,25 $) quand la clé OpenRouter le permettra.
- Design validé : `docs/design/2026-09-23-cent-ans-design.md`.

## Ce qui fonctionne
- `core/` : workspace Rust (data-model, sim-campaign, sim-battle, ai, godot-bridge). 183 tests, clippy propre.
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
- Chronique (M10, partie 1) : moteur d'événements piloté par `data/events/` (48 événements sourcés :
  26 historiques datés et conditionnels — L'Écluse, Crécy, Calais, Peste noire, Poitiers, Étienne Marcel,
  Jacquerie, Brétigny, Grandes Compagnies, Ciompi, révolte des Paysans, Maillotins, folie de Charles VI,
  Armagnacs et Bourguignons, Constance, Azincourt, Troyes, Jeanne d'Arc, Arras, Castillon… — et 22
  aléatoires à choix), conditions et effets typés, choix de l'IA par poids, décisions du joueur (fenêtre
  « Chronique », 2 tours avant choix d'office), ordre `choose_event_option`, vague de Peste noire du sud
  vers le nord sur 12 tours. 13 tests M10, smoke `_run_chronicle`. Capture : `docs/img/godot-chronicle.png`.
- Correctif M4 : les effets du gouverneur s'appliquent désormais à la population et aux impôts.

- IA de campagne stratégique (M9, partie campagne) : crate `ai` (`ai::plan_turn`), utilisée par le pont pour toutes les factions IA. Objectifs par armée (défense des provinces menacées, sièges des provinces les plus précieuses, chevauchées des factions agressives, regroupement, retraite), fusion des armées, budget militaire (60 % du revenu en guerre, 30 % en paix), recrutement de la meilleure unité par livre, construction par rendement, impôts selon la guerre et l'ordre public, licenciement en cas de dette, gouverneurs, généraux, compétences, mariages ; diplomatie (M5) et recherche (M6) réutilisées. 9 tests. Sonde : `cargo run --release -p ai --example ai_probe` (100 tours, 0 % d'ordres refusés côté France). IA de bataille : voir « IA de bataille (M9 § 2) » plus bas.

- Sièges de campagne (M8, partie campagne) : vivres de la place (famine → capitulation), brèche ouverte par les engins de siège, ordre d'assaut (bouton dans le panneau d'armée avec estimation des chances), sortie de la garnison, IA qui donne l'assaut quand les chances dépassent 65 %. 7 tests M8.
- Batailles de siège 3D (M8 § 2) : enceinte polygonale (courtines, tours, porte, place centrale) dont
  l'épaisseur, la hauteur et la solidité suivent les fortifications, brèches de campagne reportées ;
  garnison sur le chemin de ronde (tir et couvert), échelles lentes et vulnérables, tours de siège qui
  déposent l'infanterie sur le rempart, bélier contre la porte, engins qui abattent les pans pendant la
  bataille, défenseurs qui abandonnent le rempart en déroute ; victoire par la déroute de la garnison ou
  la place centrale tenue 60 s ; les chevaliers mettent pied à terre. L'assaut du joueur (ou contre lui)
  passe par le dialogue « Livrer l'assaut / Résolution automatique » ; victoire = prise de la place.
  Godot : murailles, tours, porte, maisons, beffrois, bélier et échelles procéduraux, dégâts visibles.
  10 tests de siège + 3 tests M8 (campagne), smoke § 11. Capture : `docs/img/godot-siege-battle.png`.
- IA de bataille (M9 § 2, `sim-battle/src/ai.rs`) : rôles (ligne, tireurs, ailes, réserve), posture
  défensive sur hauteur avec pieux quand le camp est plus faible, duel d'archers puis engagement,
  archers qui se replient au contact, cavalerie qui charge flancs exposés et tireurs isolés et poursuit,
  réserve qui comble les brèches, faces aux attaques de flanc, retraits ; plans de siège pour les deux
  camps ; décisions toutes les 2 s, déterministe. Rééquilibrage : batailles d'IA de ≈ 7-8 min, l'assaillant
  gagne 4-5 fois sur 10 à forces égales. 6 tests.

- Objectifs et fin de partie (M10) : objectifs historiques par faction jouable dans `data/factions/*.json` (`victory`) — France : bouter les Anglais, tenir Paris et Reims, reprendre la Guyenne, soumettre la Bourgogne (1453) ; Angleterre : sacre à Reims, héritage Plantagenêt, 15 provinces du royaume, soumettre l'Écosse (1453) ; Bourgogne : indépendance, Pays-Bas, lien lorrain (1477). Victoire, défaite ou fin de campagne avec score ; panneau Objectifs (O ou Menu), écran de fin, objectifs sur les cartes du menu de départ. 5 tests.
- Batailles (M7) : `core/crates/sim-battle` simule au pas fixe de 0,1 s un champ procédural 1200 × 800 m (collines selon le terrain de la province, forêts, boue, rivière à deux gués), la météo de saison (pluie : arcs et arbalètes −40 %, brouillard : portée −30 %, neige), des régiments en ligne/colonne/schiltron/coin avec moral, fatigue, munitions, charge, flancs (+50 %) et dos (+100 %), piques contre cavalerie, pieux des archers, déroute et ralliement, aura et mort du général, et une IA minimale. Déterministe (même graine + mêmes ordres aux mêmes ticks = même bataille), 14 tests. `sim-campaign` met les batailles du joueur en attente (`pending_battles`, réglage `interactive_battles`), fournit `battle_setup`, applique `resolve_pending_battle` (pertes, moral, captures, général tombé, XP/traits M4, retraite) ou `auto_resolve_pending` ; les restes sont auto-résolus au tour suivant ; 8 tests M7. Godot : `scenes/battle/` (terrain maillé, arbres, soldats en MultiMesh par camp et famille, bannières, caméra RTS, sélection rectangle, ordres clic droit / glisser-droit, pause, vitesses ×1/×2/×4, HUD parchemin, écran de fin). 2 × 20 régiments de 120 soldats : 60 FPS (vsync), ~140 FPS sans vsync sur M4 Pro. Captures : `docs/img/godot-battle.png`, `docs/img/godot-battle-dialog.png`. Sonde : `cargo run -p sim-battle --example probe -- ai`.

- Distribution : `tools/export_macos.sh` produit `export/Cent Ans.app` autonome (données embarquées, signature ad hoc) ; testé : la campagne démarre sur la vraie simulation. Aide en jeu (F1).
- Assets (M10, partie 2) : écus procéduraux des 16 factions (`cent-ans assets heraldry`, Pillow,
  interprétation simple du blasonnement), 10 effets sonores et 3 musiques modales de 72-80 s
  (`cent-ans assets audio`, numpy : Karplus-Strong, vièle, orgue portatif, chœur à formants, cloche ;
  OGG Vorbis via ffmpeg), 7 modèles low-poly glTF (`cent-ans assets models`, Blender headless,
  116 à 618 triangles : château, ville fortifiée, village, cathédrale, porte-étendard, camp de siège,
  cogue), portraits OpenRouter (`cent-ans assets portraits`, idempotent, enveloppe de lot, `--dry-run`
  hors ligne). Godot : autoload `AudioDirector` (bus Musique/Effets, volumes dans `user://settings.cfg`,
  menu de départ et entrée « Son… » du menu de carte, musique campagne/guerre/cour, effets de clic,
  panneaux, fin de tour et événements), `PortraitLoader` (portraits, écus en repli), `ModelLibrary`
  (villes et armées 3D, bannière teintée, repli sur les marqueurs). Tout fonctionne sans assets.
  Captures : `docs/img/godot-portraits.png`, `docs/img/godot-campaign-models.png`.

- Icônes et infobulles (F2) : 106 icônes SVG de game-icons.net (CC BY 3.0, attribution dans
  `CREDITS.md`) teintées encre sépia par `cent-ans assets icons` (cache local, hors ligne possible,
  gratuit), 144 identifiants dans `game/assets/icons/icons.json` : 13 unités, 26 bâtiments,
  9 ressources, 33 technologies (+ 2 familles), 4 classes, jauges, barre du haut (trésor, revenu,
  recherche, 4 saisons, diplomatie, chronique, cour, technologies, fin du tour), branches de
  compétences et catégories de traits, replis par catégorie. Autoload `IconLibrary`
  (`get_icon(id)`, BBCode `[img]`), infobulles riches parchemin (`RichTooltip` + `RichButton`,
  `RichPanel`, `RichLabel`, `IconChip`) : unité (coût, entretien, levée, stats, forces/faiblesses,
  capacités, prérequis, refus), bâtiment (coût, matériaux, durée, entretien, effets, prérequis),
  technologie (coût effectif, date historique, effets, déblocages, prérequis), ressource, classe,
  jauge (explication), trait, compétence. Intégrées : barre du haut (boutons Cour, Technologies,
  Diplomatie en icône seule), province (ressources, classes et jauges, bâtiments, constructible,
  garnison, recrutement), armée (cartes d'unité), bataille (cartes), arbre des technologies,
  cour, fiche personnage, faction. Smoke `_run_icons`, 9 tests pytest. Captures (après) :
  `docs/img/godot-icons-{hud,city,tooltips,tech,character,court,faction,battle}.png` (avant :
  `godot-campaign-hud.png`, `godot-city-panel.png`, `godot-tech-tree.png`, `godot-skill-tree.png`,
  `godot-court.png`, `godot-faction-panel.png`, `godot-battle.png`). Capture des infobulles :
  `--stage=tooltips`.
- Écrans et flux (F3) : menu de départ illustré par une carte ancienne rendue depuis `data/map/`
  (`cent-ans assets menu-art`), écran de chargement à étapes, menu pause (Échap), réglages persistés
  (autoload `Settings` : affichage, interface, caméra, sauvegarde auto, confirmation de fin de tour,
  batailles 3D ou auto, volumes), emplacements de sauvegarde avec vignette et fiche, sauvegarde
  automatique tournante sur 3 emplacements, « Continuer », rapport de saison cliquable, alertes
  persistantes, crédits. Smoke § 12 « flow ». Voir `docs/godot-map.md` § Écrans et flux. Captures :
  `docs/img/godot-start-menu.png`, `godot-loading.png`, `godot-flow-*.png`, `godot-credits.png`.
- Règles inertes branchées (F1, détail et choix dans `docs/wip/f1-effects.md`) : effets de bâtiments
  `Garrison` (la ville paie une part de l'entretien de sa garnison et la renforce), `RecruitCost`, `Supply`,
  ciblage par classe sociale et par famille d'unités ; effets de technologies `army_upkeep`,
  `army_experience` (recrues aguerries), `recruit_cost`, `movement` (trains de siège plus lents, artillerie
  de campagne), `production`, `siege_resistance`, `fortification_level`, `wealth`, `prestige` (annuel) ;
  surplus de recherche reporté ; traits `research_civil`/`research_military`, `Diplomacy` (attitude),
  `Intrigue` (captures), `Loyalty` (vassaux, noblesse), `Movement` et `Supply` du général ; armées alliées de
  la province dans les batailles rangées (auto-résolution, `battle_setup`, résultat 3D réparti) ; chronique :
  capture et rançon, événements programmés (catégorie `chained`, effet `schedule_event`), mariage historique,
  Jeanne de Bourbon et Charles VI (naissance 1368, folie ciblée). Rééquilibrage : richesse de base +8 par
  classe. 25 tests (`tests/f1_effects.rs`) + 1 test `data-model`.

- Guerre de Cent Ans vivante (F4, `docs/design/m9-ai.md` § 4) : guerres de prétention (l'Angleterre presse
  sa prétention à la couronne, la France ses provinces revendiquées, dès la fin des trêves), cobelligérance
  et alliances contre les rivaux, appel aux armes (`answers_call_to_arms`), vassal opportuniste, redditions,
  trésors dormants dépensés, licenciement anticipé, choix d'événements selon les moyens, mariages de l'IA,
  derniers bastions épargnés ; Pierre Ier de Portugal et Amédée VI de Savoie (héritiers de 1337), souverain
  vaincu tué à 1 %. Tests : `ai/tests/f4_war.rs` (7), `sim-campaign/tests/f4_succession.rs` (4).

### Équilibrage F4 — sonde `century_probe` (5 graines × 464 tours, 1337-1453)

| Mesure (moyenne des 5 graines, [min-max]) | Avant F4 | Après F4 | Cible |
|---|---|---|---|
| France-Angleterre en guerre | 19 % [6-61] | 72 % [55-78] | ≥ 55 % |
| Phases de guerre distinctes | 1,4 [1-3] | 6,6 [4-8] | ≥ 3 |
| Batailles impliquant FR ou EN / décennie | 1,9 [0,8-4,4] | 29,6 [19-44] | ≥ 3 |
| Provinces prises (siècle) | 50 | 383 | la carte bouge |
| Majeures (EN, FR, BOU, ÉCO) vivantes en 1400 | 3/4 partout (Écosse détruite 5/5) | 4/4 partout | majorité des graines |
| Banqueroutes / faction / décennie | 1,07 [0,91-1,51] | 0,93 [0,67-1,28] | < 1 |
| Pire faction (banqueroutes / décennie) | Suède, Suisse 7-11 | Écosse 5-16 | — |
| Trésor max après 1350 (saisons de revenu, pire faction) | 25 à 37 779 | 14 à 25 | ≤ 8 |
| Factions > 8 saisons plus de 4 tours après 1350 | 21-23 | 4-9 | 0 |
| Mariages entre factions (siècle) | 57-88 ¹ | 117-147 | > 0 |
| Appels aux armes honorés / refusés | 0-2 / 0 | 114-182 / 5-12 | la plupart |
| Ordres France refusés | 0-2 % | 0-0,3 % | 0 % |
| Durée d'une campagne (release, 5 en parallèle) | ≈ 4,5 s | ≈ 13 s | ≲ 15 s |

¹ Avant F4, les « mariages » de l'IA épousaient surtout des veuves hors d'âge (Isabelle de France et Renaud II de
Gueldre, 1337) ; F4 cherche l'âge fécond, ±15 ans, les maisons régnantes amies. Le trésor est rapporté au revenu
moyen des 8 dernières saisons ; dans la sonde la France (joueur) répond aux offres comme l'IA.
Écarts : trésors dormants encore au-dessus de 8 saisons par pointes (France, Castille, Empire après une paix
lucrative) ; l'Écosse, réduite à Fife, vit à crédit (≈ 60 % des banqueroutes) ; Flandre-Angleterre ne se noue
jamais (la Flandre reste vassale de la France sauf révolte), Bourgogne-Angleterre jamais (pas de défection
observée).

## Limites connues
- F2 : `GameDataStore` n'expose pas les définitions d'unités, bâtiments, ressources et technologies ;
  les infobulles lisent ces JSON de `data/` via `GameCatalog` (affichage seul ; coûts effectifs,
  disponibilités et refus viennent de `CampaignSim`). À remplacer par des accesseurs Rust
  (`get_unit_type`, `get_building`, `get_resource`). Forces/faiblesses d'unité : statistiques à
  ±30 % de la moyenne des types d'unités (heuristique d'affichage). Les infobulles des traits
  n'ont qu'une icône par catégorie, celles des compétences une par branche.
- M10 assets : **1 portrait sur 50** généré (`chr_afonso_iv`) : la clé OpenRouter a atteint sa limite
  mensuelle propre (100 $, consommée par d'autres usages ; 0,04 $ restants alors qu'un portrait coûte
  0,0455 $ réels). Relancer `uv run --project tools cent-ans assets portraits` après la remise à zéro
  mensuelle (≈ 2,25 $ pour les 49 restants) ; en attendant, la cour affiche l'écu de la faction.
- M10 assets : headless, `AudioDirector` charge les flux sans les jouer (le pilote factice fuit les lectures OGG).
- `get_faction_summary` renvoie 0 pour projected_income/upkeep avant le premier tour (champs mis en cache en fin de tour) ; l'interface utilise `get_faction_economy` qui calcule à la volée.
- Équilibrage (M10) : frais de cour et d'administration = 8 % du revenu + 1 % par province (plafond 35 %) + 3 % du trésor au-delà de huit saisons de revenu (F4 : 20 % au-delà de six saisons) ; débarquement en terre hostile (mouvement épuisé, −5 % d'hommes, −10 l'hiver, −10 de moral) ; l'IA n'envahit par mer que les provinces qu'elle revendique, rembourse ses dettes en 20 tours (licenciements groupés, tribut compris) ; les guerres contre une faction disparue prennent fin. Sur 5 graines × 464 tours : l'Angleterre survit partout (revenu ×2 à ×3), la France domine, l'Empire garde un trésor élevé (≈ 15 saisons de revenu) : à surveiller.
- L'IA minimale recrute une unité par tour et thésaurise ; l'IA complète est M9.
- La population ne varie pas encore (M3).
- Factions manquantes (Anjou-Provence, Grenade, Hollande-Hainaut, Brabant, Gueldre, Venise, Florence…) remplacées par la faction la plus proche, voir `docs/design/provinces-1337.md`.
- L'addon Blender MCP exige Blender ouvert en mode graphique ; le fallback headless est `tools/cent_ans_tools/blender.py`.

- L'IA ne propose pas encore de mariages (M9).
- M5 : l'IA diplomatique est volontairement prudente (peu de déclarations de guerre) ; la guerre de Cent Ans peut se conclure tôt par une paix blanche. Les noms des maisons générées viennent de la capitale ; le Portugal n'a pas de liste de prénoms dédiée.
- M6 : le mock GDScript n'a pas de technologies.
- F1 : restent sans effet `recruit_slots` (bâtiments), la `Piety` des bâtiments et des traits (la piété du
  souverain ne bouge qu'avec les événements) et `army_armor`/`army_ranged` des bâtiments (buttes de tir,
  armurerie : seuls les bonus des technologies s'appliquent en bataille). Les assauts de siège restent à
  deux (armée assiégeante contre garnison) : les alliés ne rejoignent que les batailles rangées. L'interface
  n'affiche pas encore le geôlier d'un captif ni la ventilation par classe des effets (exposée par le pont :
  `effects.by_class`). Pas d'ordre de rançon pour le joueur : les captifs sont libérés par les événements.
- Les mariages ne sont pas inscrits au journal de la simulation (ordres immédiats) : l'interface affiche un message ; l'IA ne marie encore personne (M5).
- Batailles (M7) : pas de collisions entre régiments amis ; ligne de vue simplifiée (relief, forêts) ; les engins de siège tirent comme des archers lourds en bataille rangée.
- Sièges 3D (M8) : pas de vrai cheminement (les ordres contournent une seule ouverture à la fois ; un
  ordre à travers la ville entière peut longer un mur) ; les murs bloquent par le centre des régiments,
  leurs rectangles peuvent déborder sur la maçonnerie ; tours de la muraille décoratives (pas de tir
  depuis les tours) ; bélier implicite pour tout assiégeant ; pas de sortie de la garnison pendant la
  bataille ; maisons décoratives (pas d'obstacle). Les assauts d'IA sont courts (3-6 min) face à une
  petite garnison.
- IA de bataille (M9) : pas de manœuvre d'encerclement coordonnée ni d'usage du relief en attaque ;
  l'IA ne change pas de formation (schiltron) d'elle-même.

- M10 : les événements globaux (Peste noire, Constance) attendent le choix du joueur : la vague de peste
  ne démarre qu'une fois la décision prise (ou expirée). La folie vise Charles VI : si la chronologie
  diverge (Charles V non marié à Jeanne de Bourbon, Charles VI mort ou ne régnant pas en 1392-1394), elle
  n'a pas lieu. Brétigny garde le repli « Poitiers a eu lieu » quand Jean II n'est plus captif.

## Commandes
- Build + tests : voir `CLAUDE.md`.
- Budget : `cd tools && uv run cent-ans budget show` (dépensé : 0,00 $ / 50 $).
