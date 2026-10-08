# État de l'application

Dernière mise à jour : 2026-09-24 (session 5 : portraits, miniatures, illustrations, G4 ; session 4, finalisation F1-F9 et G1-G2 ; sessions parallèles « visual »,
« ui-tw » et « historien »).

## Où en est-on
- **Jeu complet et finalisé** : campagne 1337-1453 (1477 pour la Bourgogne) jouable de bout en bout avec les
  trois factions, 29 factions au total, 117 événements de chronique dont des chaînes, IA qui mène une vraie
  guerre de Cent Ans (France-Angleterre en guerre 54-76 % du siècle, 3 à 8 phases, alliances historiques),
  batailles et sièges 3D avec phase de déploiement, rendu semi-réaliste, icônes et infobulles partout, menu
  illustré, réglages, sauvegardes automatiques, rapport de saison, alertes, tutoriel, encyclopédie (L), codex
  historique (K), manuel (`docs/manuel.md`), crédits, export macOS vérifié (`tools/export_macos.sh`).
- **Victoire** : tous les objectifs historiques tenus `hold_turns` saisons d'affilée (France 20, Angleterre et
  Bourgogne 12). Sonde : `cargo run --release -p ai --example playthrough [graine]`.
- **Qualité** : 310 tests Rust, 87 tests Python, smoke Godot 20 étapes, clippy propre.
- **Art** : 88/88 portraits, 117/117 miniatures de chronique (bandeau de la fenêtre de chronique,
  `cent-ans assets event-art`), 116 illustrations de l'encyclopédie (unités, bâtiments, technologies,
  factions, `cent-ans assets illustrations`), miniature en tête des 231 fiches du Codex (84 générées par
  `cent-ans assets codex-art`, les autres reprennent l'image de leur `entity` ; capture
  `docs/img/codex-window.png`) ; 19,20 $ dépensés sur 50 $ (`docs/budget.md`).
- **Reste** : écarts d'équilibrage documentés plus bas (tableaux F4, G2, G4, G5) ; G5 : voisinage réel
  (graphe de la carte), 4 grandes factions en vie en 1400 sur 40/40 graines.
- Plan et suivi de la finalisation : `docs/design/v2-finalisation.md`, `docs/archive/chantiers.md`.
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
- Finition de la simulation de bataille (F5a, `m7-battles.md` et `m8-sieges.md` § F5) : séparation
  douce des régiments amis, formations de l'IA (schiltron, coin, colonne) et flanc coordonné, phase de
  déploiement (zones, `deploy_unit`, `start_battle`, IA par rôles ; exposée au pont), sièges avec
  maisons et rues, cheminement A*, tir des tours et sortie de la garnison. 8 tests (`f5.rs`).
- Correctifs de la simulation de bataille (F5d, `m7-battles.md` et `m8-sieges.md` § F5d) : la bataille
  de démonstration arrive au contact (75 s, jamais avant) — un assaillant nettement plus fort abrège le
  duel de tir et lance sa cavalerie, l'IA ne s'arrête ni ne se déploie plus en eau profonde ; renforts
  échelonnés au-delà de 20 régiments par camp (`reserve` au pont) ; escalade sans brèche ramenée à 3/6
  (tir des tours allégé) ; erreurs de déploiement au nom du régiment. 6 tests (`f5d.rs`, + 2 traces ignorées).
- Déploiement et maisons de siège dans Godot (F5c, `m7-battles.md` § F5) : phase de déploiement avant
  chaque bataille du joueur (zone au sol, clic droit / glisser-droit, refus en message, « Commencer la
  bataille » ou Entrée), maisons de siège sur les disques de la simulation, sortie de la garnison
  signalée. Captures `docs/img/godot-battle-deploy.png`, `docs/img/godot-siege-f5.png`.

- Objectifs et fin de partie (M10) : objectifs historiques par faction jouable dans `data/factions/*.json` (`victory`) — France : bouter les Anglais, tenir Paris et Reims, reprendre la Guyenne, soumettre la Bourgogne (1453) ; Angleterre : sacre à Reims, héritage Plantagenêt, 15 provinces du royaume, soumettre l'Écosse (1453) ; Bourgogne : indépendance, Pays-Bas, lien lorrain (1477). Victoire, défaite ou fin de campagne avec score ; panneau Objectifs (O ou Menu), écran de fin, objectifs sur les cartes du menu de départ. 5 tests.
- Batailles (M7) : `core/crates/sim-battle` simule au pas fixe de 0,1 s un champ procédural 1200 × 800 m (collines selon le terrain de la province, forêts, boue, rivière à deux gués), la météo de saison (pluie : arcs et arbalètes −40 %, brouillard : portée −30 %, neige), des régiments en ligne/colonne/schiltron/coin avec moral, fatigue, munitions, charge, flancs (+50 %) et dos (+100 %), piques contre cavalerie, pieux des archers, déroute et ralliement, aura et mort du général, et une IA minimale. Déterministe (même graine + mêmes ordres aux mêmes ticks = même bataille), 14 tests. `sim-campaign` met les batailles du joueur en attente (`pending_battles`, réglage `interactive_battles`), fournit `battle_setup`, applique `resolve_pending_battle` (pertes, moral, captures, général tombé, XP/traits M4, retraite) ou `auto_resolve_pending` ; les restes sont auto-résolus au tour suivant ; 8 tests M7. Godot : `scenes/battle/` (terrain maillé, arbres, soldats en MultiMesh par camp et famille, bannières, caméra RTS, sélection rectangle, ordres clic droit / glisser-droit, pause, vitesses ×1/×2/×4, HUD parchemin, écran de fin). 2 × 20 régiments de 120 soldats : 60 FPS (vsync), ~140 FPS sans vsync sur M4 Pro. Captures : `docs/img/godot-battle.png`, `docs/img/godot-battle-dialog.png`. Sonde : `cargo run -p sim-battle --example probe -- ai`.
- HUD de bataille F5b (audit UI § 3.2) : cartes compactes rangées par « bataille » (avant-garde, bataille, arrière-garde), groupes Ctrl+1..9 / 1..9, vitesses en boutons-icônes en bas à droite (+ / − au clavier), minicarte cliquable, aide F1 au lieu de la ligne d'aide permanente. Détail : `docs/design/m7-battles.md` § F5b ; capture `docs/img/godot-battle-f5.png`.

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
- Règles inertes branchées (F1, détail et choix dans `docs/archive/chantiers.md`) : effets de bâtiments
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

- Chronique enrichie (F7b) : 90 événements (54 historiques, 8 chaînés, 28 aléatoires). 40 nouveaux,
  sourcés : paix des Scaliger, Laupen, Valdemar IV, siège de Tournai → trêve d'Esplechin, Salado,
  succession de Bretagne → Hennebont, banqueroute des Bardi, Charles IV roi des Romains, Neville's Cross
  (capture de David II) → rançon de Berwick, Cola di Rienzo → chute du tribun, achat du Dauphiné, combat
  des Trente, Bulle d'or, Cocherel, Auray, Nájera, appel des seigneurs gascons (reprise de la guerre, 1369),
  Auld Alliance renouvelée, La Rochelle, lords Appelants, Nicopolis → rançon de Jean de Nevers, Harfleur
  (1415), Montereau → alliance anglo-bourguignonne → Troyes, Verneuil, Jeanne d'Arc → sacre de Reims, Patay,
  procès de Rouen, Fougères (1449), Formigny ; 6 aléatoires pour les factions F7 (galères vénitiennes,
  banque florentine, piquiers suisses, argent de Kutná Hora, harengs de Scanie, razzias grenadines).
  Campagne simulée 1337-1453 (graine fixe, IA) : les 27 nouveaux historiques et les 7 étapes chaînées se
  déclenchent. Un aléatoire réservé à une faction ne consomme plus de tirage pour les autres. 6 tests (`tests/f7_events.rs`).
- Guerre de Cent Ans vivante (F4, `docs/design/m9-ai.md` § 4) : guerres de prétention (l'Angleterre presse
  sa prétention à la couronne, la France ses provinces revendiquées, dès la fin des trêves), cobelligérance
  et alliances contre les rivaux, appel aux armes (`answers_call_to_arms`), vassal opportuniste, redditions,
  trésors dormants dépensés, licenciement anticipé, choix d'événements selon les moyens, mariages de l'IA,
  derniers bastions épargnés ; Pierre Ier de Portugal et Amédée VI de Savoie (héritiers de 1337), souverain
  vaincu tué à 1 %. Tests : `ai/tests/f4_war.rs` (7), `sim-campaign/tests/f4_succession.rs` (4).
- Dernières règles inertes (G1, `docs/archive/chantiers.md`) : places de recrutement par province (2, +1 dans la
  capitale, + `recruit_slots` des bâtiments et techs ; refus « file de recrutement pleine ») ; piété des traits
  (piété effective : faveur pontificale, hérésie) et des bâtiments (+1 par an au souverain par 10 points,
  plafond 3) ; armurerie et buttes de tir équipent les unités levées dans la province (armure/tir stockés sur
  l'unité, auto-résolution et `battle_setup`) ; effet d'événement `transfer_province` (achat du Dauphiné,
  traité de Guérande, Formigny) ; ordre `release_captive` (libérer contre rançon), fiche personnage « Captif
  de … — rançon N livres » avec « Payer la rançon » / « Libérer contre rançon » ; alliés de la province dans
  les assauts de siège. 10 tests (`sim-campaign/tests/g1_rules.rs`).

### Équilibrage F4 — sonde `century_probe` (5 graines × 464 tours, 1337-1453)

| Mesure (moyenne des 5 graines, [min-max]) | Avant F4 | Après F4 | Cible |
|---|---|---|---|
| France-Angleterre en guerre | 19 % [6-61] | 73 % [55-83] | ≥ 55 % |
| Phases de guerre distinctes | 1,4 [1-3] | 7,0 [5-8] | ≥ 3 |
| Batailles impliquant FR ou EN / décennie | 1,9 [0,8-4,4] | 30,2 [19-43] | ≥ 3 |
| Provinces prises (siècle) | 50 | 375 | la carte bouge |
| Majeures (EN, FR, BOU, ÉCO) vivantes en 1400 | 3/4 partout (Écosse détruite 5/5) | 4/4 partout | majorité des graines |
| Banqueroutes / faction / décennie | 1,07 [0,91-1,51] | 0,92 [0,67-1,27] | < 1 |
| Pire faction (banqueroutes / décennie) | Suède, Suisse 7-11 | Écosse 5-16 (réduite à Fife) | — |
| Trésor max après 1350 (saisons de revenu, pire faction) | 25 à 37 779 | 13 à 25 | ≤ 8 |
| Factions > 8 saisons plus de 4 tours après 1350 | 21-23 | 5-8 | 0 |
| Mariages entre factions (siècle) | 57-88 ¹ | 121-147 | > 0 |
| Appels aux armes honorés / refusés | 0-2 / 0 | 112-191 / 5-12 | la plupart |
| Ordres France refusés | 0-2 % | 0-0,1 % (`ai_probe` 100 tours : 0 %) | 0 % |
| Durée d'une campagne (release, 5 en parallèle) | ≈ 4,5 s | ≈ 13 s (≈ 30 s sur machine chargée) | ≲ 15 s |

¹ Avant F4, les « mariages » de l'IA épousaient surtout des veuves hors d'âge (Isabelle de France et Renaud II de
Gueldre, 1337) ; F4 cherche l'âge fécond, ±15 ans, les maisons régnantes amies. Le trésor est rapporté au revenu
moyen des 8 dernières saisons ; dans la sonde la France (joueur) répond aux offres comme l'IA.
Écarts : trésors dormants encore au-dessus de 8 saisons par pointes (France, Castille, Empire après une paix
lucrative) ; l'Écosse, réduite à Fife, vit à crédit (≈ 60 % des banqueroutes) ; Flandre-Angleterre ne se noue
jamais (la Flandre reste vassale de la France sauf révolte), Bourgogne-Angleterre jamais (pas de défection
observée).

### Alignement historique G2 — sonde `century_probe` (5 graines × 464 tours)

| Mesure (graines 1 à 5) | Avant G2 | Après G2 | Cible |
|---|---|---|---|
| Banqueroutes de l'Écosse / décennie | ≈ 35 (4 graines sur 5) | 0,1-1,6 | < 3 |
| Banqueroutes / faction / décennie | 1,70 [0,47-2,29] | 0,52 [0,30-0,69] | < 1 |
| Angl.-Flandre (part des tours de guerre FR-EN) | 0 % partout | 0 / 100 / 0 / 100 / 0 % (moy. 40 %) | penche vers l'Angleterre |
| Angl.-Hainaut (idem) | 0 % sauf 62 % (graine 5, tous tours) | 0 / 97 / 22 / 0 / 69 % (moy. 38 %) | 20-60 % |
| Angl.-Brabant (idem) | 0 % sauf 70 % (graine 5, tous tours) | 0 / 14 / 0 / 0 / 0 % (moy. 3 %) | 20-60 % |
| Bourg.-Angl. | 0 % | 0 % (Angleterre dominante 0-8 % des tours) | une partie des graines |
| Trésor max après 1350 (saisons, pire faction) | 13-55, médiane 24 | 9,9-14,3 hors Empire graine 5 ¹, médiane 13,7 | ≤ 12, médiane ≤ 8 |
| Factions > 8 saisons plus de 4 tours après 1350 | 2-5 | 1-3 | 0 |
| France-Angleterre en guerre | 70 % [67-76] | 58 % [45-76] | ≥ 55 % |
| Ordres France refusés | 0-0,2 % | 0-0,1 % (`ai_probe` 100 tours : 0 %) | 0 % |

¹ Empire assiégé partout, revenu moyen quasi nul : le rapport explose (15 974) pour un trésor à peine au-dessus
de 10 000 livres. Écarts : Brabant rarement anglais (la marge d'attitude n'est franchie que si le Hainaut est
déjà allié) ; la défection bourguignonne est en place et testée mais ne se déclenche jamais faute de domination
anglaise (au plus 6 provinces du royaume tenues à la fois, seuil 8) ; trésors encore 10-14 saisons par pointes
(royaumes réduits à une province, revenus effondrés par les sièges) ; la guerre FR-EN recule un peu (graines 1
et 3 sous 55 %) ; Bourgogne jouée par l'IA (`playthrough 1`) toujours à 2 provinces en 1478.

### Bourgogne et Brabant G4 — sonde `century_probe` (464 tours)

Avant = `main` ac6b7c2 (G2 + colonies C1) ; après = lot G4 (`docs/design/m9-ai.md` § 6, réglages
`data/ai/alignment.json`). Graines 1 à 5, puis 40 graines (1-40) pour lisser le chaos des campagnes.

| Mesure | Avant (1-5) | Après (1-5) | Avant (40 gr.) | Après (40 gr.) | Cible |
|---|---|---|---|---|---|
| Bourg.-Angl. (graines avec alliance) | 0/5 | 5/5, dès 1419-1422 | 4/40 | 23/40, dès 1419-1442 | ≥ 2/5, XVe s. |
| Bourg.-Angl. (part des tours de guerre FR-EN) | 0 % | 16/17/3/12/39 % | 2 % | 11 % | — |
| Angl.-Brabant (part des tours de guerre FR-EN) | 0 % partout | 0/100/28/20/0 % (moy. 30 %) | 10 % | 39 % | 20-60 % |
| Angl.-Hainaut (idem) | 0/0/0/0/16 % | 0/0/79/0/0 % | 30 % | 37 % | — |
| Angl.-Flandre (idem) | 0/100/0/91/0 % | 0/100/0/72/0 % | 51 % | 46 % | — |
| France-Angleterre en guerre | 53/75/58/55/25 % (moy. 53 %) | 61/61/76/55/19 % (moy. 54 %) | 57 % | 62 % | ≥ 55 % |
| Majeures en vie en 1400 | 4/4 partout | 4/4 sauf Écosse graine 3 | 40/40 | 38/40 | 4/4 |
| Banqueroutes / faction / décennie | 0,44 [0,35-0,58] | 0,49 [0,38-0,62] | 0,47 | 0,49 | < 1 |
| Trésor max (médiane) | 9,9 ¹ | 11,1 ¹ | 10,9 | 12,8 | ≤ 12 |
| Auld Alliance (part des tours de guerre FR-EN) | 81-97 % | 3-100 % (moy. 78 %) | 93 % | 83 % | — |
| Bourg.-France (idem) | 92-100 % | 57-83 % | 94 % | 74 % | — |

¹ Médianes ; une graine par colonne a un rapport explosé (avant : Flandre graine 5, 10 790 ; après : Bourgogne graine 3, 11 133) par un revenu quasi nul.
`playthrough` 1-5 : aucune panique, 0-7 ordres refusés ; France jouée par l'IA battue en 1449 (graine 1).

Mécanismes : grief de Montereau mesuré sur les modificateurs d'opinion (le choix pondéré de l'événement
fait l'hésitation, pas un tirage) ; rupture avec les alliés que le nouveau patron tient pour rivaux (sans
quoi l'Angleterre refusait « l'allié des Écossais ») ; fiefs-rentes et toile des Pays-Bas (princes moindres
voisins d'un allié moindre déjà en guerre) ; domination relative (15 % du royaume). Écarts : deux
disparitions de majeures avant 1400 sur 40 graines (Écosse graine 3, attaque seule l'Angleterre en 1357 ;
France graine 6, écrasée en 1387 par une coalition anglaise gonflée d'alliances génériques) ; l'Auld
Alliance et la fidélité bourguignonne reculent (moins de tours alliés, par les défections et les
destructions) ; trésors un peu plus hauts. Bogue signalé : `CampaignState::are_neighbors` ne lit que les
`neighbors` des fichiers de province (6 provinces sur 132 renseignées) ; l'IA d'alignement utilise les
frontières de la carte, le correctif global reste à équilibrer.

### Voisinage réel G5 — sonde `century_probe` (464 tours)

`CampaignState::are_neighbors` suit désormais le graphe de la carte (`movement::land_neighbors`,
`data/map/provinces.geojson`) au lieu des `neighbors` des fichiers de province (6 sur 132) ; `ai::alignment::borders`
est supprimé (une seule source), la propagation de l'hérésie et `get_province().neighbors` du pont suivent le même
graphe. Réglages : `data/ai/diplomacy.json` (schéma `ai_diplomacy.schema.json`, `docs/design/m9-ai.md` § 7).
Avant = `main` f52fd92 (G4) ; « correctif seul » = adjacence réelle sans rééquilibrage ; après = lot G5.

| Mesure (40 graines) | Avant | Correctif seul | Après | Cible |
|---|---|---|---|---|
| France-Angleterre en guerre | 66 % [41-83], 22/40 dans la bande | 69 % [53-88], 24/40 | 65 % [42-85], 29/40 | 55-75 % |
| Auld Alliance (part des tours de guerre FR-EN) | 81 % [6-100] | 81 % [5-100] | 90 % [40-100] | ≥ 80 % |
| Angl.-Brabant (idem) | 35 % | 31 % | 25 % | 20-60 % |
| Bourg.-France (idem) | 78 % | 74 % | 74 % | — |
| Bourg.-Angl. (graines, XVe s.) | 22/40 | 20/40 | 21/40 (22 en tout) | ≥ 2/5 |
| Banqueroutes / faction / décennie | 0,47 [0,27-1,59] | 0,52 [0,30-1,14] | 0,43 [0,25-0,70] | < 1 |
| Appels aux armes honorés (moy. / campagne) | 170 | 736 | 350 | — |
| 4 majeures en vie en 1400 | 37/40 ¹ | 39/40 | **40/40** | 40/40 |

| Mesure (graines 1 à 5) | Avant (G4) | Après | Cible |
|---|---|---|---|
| France-Angleterre en guerre | 61/61/76/55/19 % | 62/61/56/64/64 % | 55-75 % |
| Auld Alliance (tours de guerre FR-EN) | moy. 78 % | 100/99/100/76/97 % | ≥ 80 % |
| Angl.-Brabant (idem) | 0/100/28/20/0 % | 0/100/41/0/3 % (moy. 29 %) | 20-60 % |
| Bourg.-Angl. | 5/5 | 2/5 (1421, 1422) | ≥ 2/5, XVe s. |
| Banqueroutes / faction / décennie | 0,49 | 0,51 [0,42-0,70] | < 1 |
| Majeures en vie en 1400 | 4/5 | 5/5 | 5/5 |

¹ La mesure G4 (38/40) a été refaite sur `main` f52fd92 : 37/40 (Écosse graines 26, 27, 35).
`playthrough` 1-5 (France, Angleterre, Bourgogne jouées par l'IA) : aucune panique, 0-5 ordres refusés.

Mécanismes : cobelligérance réservée aux alliés au moins aussi puissants que nous (`min_ally_power_ratio` 1 : le
prince moindre suit la grande couronne, pas l'inverse) et, sur simple frontière, aux guerres de prétentions
(`border_only_claim_wars`) — c'est ce qui ramène les appels aux armes de ×4 à ×2 ; une couronne ne cède par
traité ni sa capitale ni sa dernière province (`keep_capital`), et la paix qui lui laisse cette terre solde
quand même la guerre ; une couronne réduite à une province à elle (`cornered_provinces`) demande la paix à
chaque saison, quel que soit le score (l'Écosse traite au lieu de disparaître en 1340-1341). Écarts : les
appels aux armes restent deux fois plus nombreux qu'avant (plus de voisins, donc plus de guerres de défense
possibles) ; France-Angleterre sous 55 % sur 5 graines sur 40 (au-dessus de 75 % sur 6) ; Bourg.-Angl.
2/5 sur les graines 1-5 (5/5 en G4), 21/40 sur 40 ; l'Écosse tombe encore en 1420 sur une graine (après 1400).

## Limites connues
- F2 : `GameDataStore` n'expose pas les définitions d'unités, bâtiments, ressources et technologies ;
  les infobulles lisent ces JSON de `data/` via `GameCatalog` (affichage seul ; coûts effectifs,
  disponibilités et refus viennent de `CampaignSim`). À remplacer par des accesseurs Rust
  (`get_unit_type`, `get_building`, `get_resource`). Forces/faiblesses d'unité : statistiques à
  ±30 % de la moyenne des types d'unités (heuristique d'affichage). Les infobulles des traits
  n'ont qu'une icône par catégorie, celles des compétences une par branche.
- M10 assets : les images générées sont des JPEG/PNG chargés à la volée (`PortraitLoader.load_texture`) ;
  tout le jeu fonctionne sans elles (écu de faction ou bandeau masqué en repli).
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
- F1 : l'interface n'affiche pas encore la ventilation par classe des effets (exposée par le pont :
  `effects.by_class`).
- G1 : la bataille 3D applique désormais les bonus des technologies de chaque camp (moral, mêlée, tir,
  armure, par catégorie d'unité) en plus de ceux des bâtiments de la province de levée dans `battle_setup`
  (`docs/archive/chantiers.md`) ; l'IA ne libère jamais un captif contre rançon d'elle-même
  (elle paie, ou libère sur parole les simples chevaliers). Pas de faction Danemark : l'achat du Jutland
  (`evt_valdemar_iv`) reste sans cession. Le plafond de recrutement (G1) limite l'IA à 3 levées par tour
  dans sa capitale et 2 ailleurs (hors bâtiments) : trésors à resurveiller avec `century_probe`.
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
