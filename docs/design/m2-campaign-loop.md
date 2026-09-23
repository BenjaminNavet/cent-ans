# M2 — Boucle de campagne minimale jouable : spécification

Date : 2026-09-23. Objectif : jouer France, Angleterre ou Bourgogne au printemps 1337, déplacer des armées,
recruter, combattre en résolution automatique, prendre des provinces, sauvegarder. Les IA bougent un minimum.

## 1. Modèle de simulation (`core/crates/sim-campaign`)

### 1.1 État (`CampaignState`, sérialisable serde, déterministe)
- `turn`, `season`, `year`, `seed`, `rng` (ChaCha8, sérialisé), `player_faction`.
- `provinces: BTreeMap<ProvinceId, ProvinceState>` : `owner`, `controller` (occupant), `garrison` (unités), `siege: Option<SiegeState>` (assiégeant, tours restants), `unrest`, `devastation` 0-100 (chevauchée), `population` (copie évolutive des classes).
- `factions: BTreeMap<FactionId, FactionState>` : `treasury` (livres), `income_last_turn`, `at_war_with: BTreeSet<FactionId>`, `allies`, `truces: BTreeMap<FactionId, u32 turn_end>`, `alive`.
- `armies: BTreeMap<ArmyId, Army>` : `faction`, `general: Option<CharacterId>`, `location: ProvinceId`, `units: Vec<Unit>`, `movement_points` (max 3 par tour, terrain/saison modulent), `supply` 0-100, `stance` (Normal, Raid, Siege), `path: Vec<ProvinceId>` (en cours, résolu en fin de tour).
- `units: Unit` : `unit_type`, `strength` (effectif courant), `max_strength`, `experience`, `morale`.
- `characters: BTreeMap<CharacterId, CharacterState>` : `alive`, `age` (calculé), `location`, `army: Option<ArmyId>`, `skills`.
- `events: Vec<GameEvent>` (journal du tour : batailles, sièges, prises, revenus, morts), `pending_battles: Vec<BattleRequest>`.

### 1.2 Ordres (`Order`, tous validés avant application ; erreur explicite sinon)
- `MoveArmy { army, path }` : chemin de provinces adjacentes (terre ou lien maritime depuis un port), coût cumulé ≤ points de mouvement (mer = 2 points fixe par lien, requiert `port` des deux côtés). Le chemin peut dépasser : l'armée avance de ce qu'elle peut et continue au tour suivant.
- `Recruit { province, unit_type }` : requiert bâtiment/tech, faction propriétaire et contrôleur, classe sociale disponible ; coût prélevé immédiatement, l'unité apparaît en garnison au tour suivant.
- `CreateArmy { province, units_from_garrison, general }`, `MergeArmies`, `SplitArmy`, `DisbandUnit`.
- `SetStance { army, stance }`, `AssignGeneral { army, character }`.
- `EndTurn`.

### 1.3 Résolution de fin de tour (ordre fixe, `end_turn()`)
1. Ordres de l'IA (crate `ai`, fonction pure `plan_turn(&CampaignState, faction) -> Vec<Order>`).
2. Mouvement de toutes les armées, province par province, simultané par étapes : quand une armée entre dans une province avec une armée ennemie (guerre déclarée) → `BattleRequest` ; arrêt des deux. Les combats se résolvent (auto) dans cette même passe pour la v1 ; les `BattleRequest` du joueur sont exposées avant application pour une bataille 3D future (M7) — en M2 tout est auto.
3. Sièges : armée ennemie en `Siege` dans une province sans armée défenseure de terrain → siège ; garnison vide → prise immédiate ; sinon durée = 2 + niveau de fortification tours, ou assaut (bataille contre la garnison avec malus attaquant).
4. Chevauchées (`Raid`) : dévastation +30, butin, mécontentement local.
5. Économie : revenus par province (taxes par classe × richesse × (1 - dévastation)), entretien des unités, bâtiments en construction (M3, ignoré ici), trésor mis à jour ; trésor négatif → moral -10 sur toutes les unités.
6. Ravitaillement/attrition : hors territoire ami ou allié, supply -20 par tour (hiver -35) ; supply 0 → effectifs -10 %/tour. En territoire ami, supply +40.
7. Mécontentement et dévastation : décroissance lente.
8. Personnages : vieillissement, mort naturelle (probabilité par âge, déterministe via rng), remplacement du dirigeant par l'héritier (`faction.heir`, sinon plus vieux mâle de la maison, sinon faction morte si aucune province).
9. Date +1 saison ; `events` remplacé par ceux du tour.

### 1.4 Résolution automatique des batailles (`battle_auto.rs`)
Pour chaque camp : puissance = Σ unités (strength × (mêlée ou tir selon catégorie) × (1 + expérience/10)) × facteurs : moral moyen, fatigue (supply), général (commandement × 3 %), terrain (défenseur en colline/forêt +15 %, rivière à traverser -20 % pour l'attaquant), armure moyenne vs tir adverse. Ratio de puissance → pertes proportionnelles (perdant 15-40 %, gagnant 5-15 %), routes quand moral < 25, capture possible du général vaincu (10 %). Retour : `BattleResult { winner, losses, captured, retreat_to }`. Tests : symétrie, déterminisme, avantage numérique, avantage terrain.

### 1.5 Sauvegarde
`save(&self) -> String` (JSON serde, version d'état incluse), `load(json) -> Result`. Round-trip testé.
Fichiers `user://saves/<nom>.json` côté Godot.

### 1.6 Départ 1337
Construit depuis `GameData` : provinces avec propriétaire, garnisons initiales (2-4 unités selon capitale/port/frontière), une armée principale par faction jouable avec le roi comme général (Philippe VI à Paris avec ~8 unités, Édouard III à Londres ~6, Eudes IV à Dijon ~4), trésors initiaux (France 60 000, Angleterre 40 000, Bourgogne 15 000 livres ; mineurs 5 000-20 000), guerres initiales depuis `factions/*.json` `relations` (France–Angleterre en guerre, Écosse alliée de France, etc.).

## 2. API GDExtension (`CampaignSim`, `core/crates/godot-bridge`)
- `new_campaign(data_dir: String, player: String, seed: int) -> bool`, `save_to_string() -> String`, `load_from_string(json) -> bool`.
- `get_turn()`, `get_date_label()`, `get_player_faction()`.
- `get_faction_summary(id) -> Dictionary { treasury, income, at_war_with[], allies[], provinces_count, armies_count, alive }`.
- `get_province_state(id) -> Dictionary { owner, controller, garrison[ {unit_type, strength, max_strength, morale} ], siege { attacker, turns_left }?, unrest, devastation, population_total }`.
- `get_army_ids() -> PackedStringArray`, `get_army(id) -> Dictionary { faction, general, general_name, location, units[...], movement_points, supply, stance, path[] }`.
- `get_reachable(army_id) -> Dictionary { province_id: cost }` (Dijkstra borné par les points de mouvement, pour l'aperçu de chemin) et `find_path(army_id, target) -> PackedStringArray`.
- `get_recruitable(province_id) -> Array[Dictionary { unit_type, name, cost, upkeep, available: bool, reason }]`.
- `submit_order(order: Dictionary) -> Dictionary { ok: bool, error: String }` avec `{"type": "move_army", "army": id, "path": [...]}` etc. (noms snake_case identiques aux variantes Rust).
- `end_turn() -> Array[Dictionary]` (événements du tour : `{ kind, text_fr, province?, army?, faction? }`).
- `get_events() -> Array[Dictionary]`.
- `GameDataStore.load_heightmap_u16(path) -> PackedByteArray` (décodage PNG 16 bits côté Rust, little-endian) pour remplacer le décodage GDScript.

## 3. Interface Godot (`game/`)
- Écran de démarrage : choix de la faction (3 cartes avec blason texte, description courte), bouton « Commencer », « Charger ».
- Barre supérieure : blason/nom de faction, trésor, revenu, date, bouton « Fin du tour » (raccourci Entrée), menu (sauver, charger, quitter).
- Carte : couleurs de faction depuis `GameDataStore` (plus de palette de secours), marqueurs d'armées (bannière avec couleur de faction, chiffre d'unités, halo si sélectionnée), clic gauche sélection (armée prioritaire sur province), clic droit sur province → ordre de déplacement avec aperçu du chemin (ligne + coût) au survol quand une armée est sélectionnée, provinces atteignables surlignées.
- Panneau armée : général, unités (nom, effectif/max, moral), posture (Normal/Raid/Siège), points de mouvement, ravitaillement.
- Panneau province : (existant) + propriétaire réel, contrôleur, garnison, siège, mécontentement, dévastation, bouton « Recruter » → liste des unités recrutables avec coût et raison si indisponible, bouton « Former une armée » depuis la garnison.
- Journal des événements du tour (panneau déroulant en bas à gauche), notifications de bataille (résumé : lieu, camps, pertes, vainqueur).
- Sauvegarde/chargement en JSON dans `user://saves/`.
- Le smoke test étendu : nouvelle campagne France, ordre de déplacement de l'armée de Philippe VI vers une province voisine, 4 fins de tour sans erreur, sauvegarde puis rechargement, égalité des dates.

## 4. IA minimale (`core/crates/ai`)
- Par faction et par tour : si trésor > coût, recruter l'unité la moins chère disponible dans la capitale ; toute armée sans ordre : si une province ennemie adjacente est faiblement défendue → s'y déplacer en `Siege`, sinon rester ou revenir vers la province amie la plus menacée. Pas de diplomatie (M5).

## 5. Critères de fin de M2
- Partie jouable de bout en bout au clavier/souris sur les trois factions, 20 tours sans crash ni erreur console.
- Tests Rust : ordres invalides refusés, mouvement/pathfinding, bataille auto, sièges, économie, sauvegarde round-trip, déterminisme sur 20 tours avec IA.
- Smoke test Godot étendu vert.
