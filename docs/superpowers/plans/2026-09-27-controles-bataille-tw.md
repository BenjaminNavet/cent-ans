# Plan d'implémentation — Contrôles de bataille façon Total War (lots CB)

Spec : `docs/superpowers/specs/2026-09-27-controles-bataille-tw-design.md` (validée).
Première action après approbation : copier ce plan dans `docs/superpowers/plans/2026-09-27-controles-bataille-tw.md`, créer la note d'orchestration `docs/archive/chantiers.md` (tableau des lots, état, prochaine étape), réserver l'ADR `docs/decisions/0095-controles-bataille-tw.md`, commit `docs:`.

## Contexte

Suite à la comparaison avec Total War: Warhammer 3, le joueur veut d'abord la même façon de commander : marqueurs de sélection, aperçu du trajet, curseur contextuel, formation au glisser, modes, caméra et vue tactique, capacités, ordres en file, alertes. Le modèle de mêlée et la difficulté ne sont pas concernés. La règle du projet s'applique : les règles vivent dans `core/`, le rendu et les entrées dans `game/`, les chiffres dans `data/`.

## Écarts entre la spec et le code, et décisions prises

L'exploration du code a révélé six écarts. Pour chacun, le plan fait un choix qui respecte l'intention de la spec, et l'ADR 0095 les consigne.

1. **Pas de navgrid en bataille de campagne.** La navgrid R3 ne sert qu'à la carte de campagne. En bataille :
   - un `Move` fixe une destination, et chaque tick appelle `route()` (`sim-battle/src/sim.rs:1497`) ;
   - en rase campagne, `route()` suit une ligne droite, avec au besoin un point de passage de gué ou de pont (`water_route`, `sim/water.rs:84`) ;
   - en siège, il utilise un A* sur des cases de 5 m (`grid_route` et `siege_route`, `sim/pathing.rs:233/272`).

   → `preview_path` extrait de `route()` une fonction pure `plan_route(side, from, to) -> Option<Vec<(f64,f64)>>` qui renvoie la **chaîne complète** des points de passage. `route()` n'en garde que le premier point, ce qui garantit l'identité avec l'ordre réel. La destination est « inaccessible » dans trois cas : hors carte, eau profonde sans gué, ou `siege_route` qui renvoie `None`.
2. **Pas de brouillard de guerre pour le joueur.** `get_units` expose tous les ennemis. Or la vue tactique ne doit montrer que les ennemis repérés.

   → CB3 ajoute un champ `spotted` (vu par au moins une unité du camp joueur) calculé dans le cœur. Il réutilise `visible()` (`sim.rs:2175`) et `seen_at`, avec une portée de vue tirée de `data/rules/battle_vision.json` ou de la règle existante si elle existe déjà. La vue normale ne change pas.
3. **« Général blessé » n'existe pas**, faute d'état de blessure.

   → L'alerte CB5 couvre « général tué ou capturé ». La blessure reste hors périmètre.
4. **Les événements du journal ne sont pas typés** : `BattleEvent{time, text_fr, side}` (`outcome.rs:86`). Le flanc ou le dos attaqué n'est jamais journalisé, bien que `Unit.flanked` existe (`unit.rs:226`).

   → CB5 ajoute un flux typé `BattleAlert{kind, time, x, z, side, unit}` à côté du journal, sans le remplacer. On l'émet aux mêmes endroits que les `log()`, plus une émission sur passage de `flanked` à vrai.
5. **La formation n'a pas de largeur stockée** : `ranks_files(n)` la déduit du type de formation (`unit.rs:459`).

   → CB1 ajoute `line_files: Option<u32>` sur `Unit`. Ce champ remplace le calcul de la formation Ligne quand il est fixé. Il n'y a pas de nouveau type de formation.
6. **Les commandes sont enregistrées pour le rejeu EP13** (`REPLAY_FORMAT = 1`).

   → Tout nouveau champ de `Command` est `#[serde(default)]`, et les nouvelles variantes sont additives : les anciens rejeux restent lisibles. Un test EP13 relit un rejeu enregistré avant CB.

**Icônes** : curseurs, modes, états, capacités, alertes et cadenas passent tous par le pipeline DA5 :
- on ajoute des entrées à `data/ui/icons_ink.json`, puis on lance `cent-ans assets ink-icons --only …` ;
- une quarantaine d'icônes, environ 2 $ au total, consignés dans `docs/budget.md` ;
- les curseurs sont dérivés en 32 px, avec le point chaud au centre, dans `game/assets/ui/cursors/`.

## Conventions communes à tous les lots

- **Une branche et un worktree par lot**, avec une cible cargo privée (`CARGO_TARGET_DIR` propre au worktree) :
  - fusion dans un worktree séparé, puis ff-only dans main ;
  - réimport Godot après chaque fusion ;
  - commits avec des chemins explicites (`git commit -- paths`) ;
  - jamais de `git stash`.
- **Déroulé d'un lot** :
  1. Squelette : API publique, fichiers vides, tests `#[ignore]`. Premier commit `wip:`.
  2. Implémentation guidée par les tests.
  3. Commit `wip:` au plus toutes les 15 min, avec la note `docs/wip/cb<lot>.md` à jour.
- **Avant chaque fusion** :
  - `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test` ;
  - `uv run --project tools pytest` (schémas) ;
  - `godot --headless --path game --script res://tests/smoke.gd`.
- **Nouvelle règle chiffrée** :
  - JSON dans `data/rules/` et schéma `data/schemas/*.schema.json` ;
  - test `tools/tests/test_<nom>_schema.py`, calqué sur `test_battle_rout_schema.py` ;
  - chargement dans le cœur par `include_str!` avec `OnceLock` et `deny_unknown_fields`, comme `sim-battle/src/rout.rs:75-84`.
- **Chiffres affichés** : ils passent par `RuleValues` (`game/scripts/ui/rule_values.gd`). Les constantes de bataille nouvelles s'ajoutent à `godot-bridge/src/data_store_rules.rs`.
- **Captures** : scripts `game/tests/cb*_shot.gd`, au plus 3 captures par lot, lues seulement par la session principale.
- **Agents** : Sonnet pour CB0, CB3, CB5 et les parties données, sans outils de capture ; la session principale se charge de CB-M et CB4 (cœur délicat).
- **Coordination CV3** :
  - CV3-2 (embuscade) modifie `sim-battle/src/sim.rs` et `sim/deployment.rs`. Attendre sa fusion avant le premier lot qui touche `sim.rs` (CB-M2).
  - CV3-6 (sondes d'équilibrage) : ne pas lancer les batailles de référence CB2/CB4 en même temps.

## Lots

### CB0 — Extraction des entrées, puis sélection rapide (`game/` seul, Sonnet)

**Fichiers**
- Nouveau : `game/scripts/battle/battle_input.gd`, un nœud enfant de la scène.
- Modifié : `battle_scene.gd`, lignes 1627-1922.

**Étapes**
1. **Test d'équivalence d'abord**, commité **avant** l'extraction : `game/tests/cb0_input_equivalence_test.gd`.
   - Il ouvre une bataille de référence et injecte une séquence scriptée par `get_viewport().push_input()`, ce qui reste neutre vis-à-vis de l'emplacement du gestionnaire.
   - La séquence couvre : clic, sélection au cadre, Maj-clic, clic droit, glisser-droit, double clic droit, clic droit sur ennemi, touches F/G/H/C/Échap/Espace/+/−, groupes Ctrl+1 puis 1, 1 et 1.
   - Il enregistre les dictionnaires passés à `issue()` : on ajoute à la scène un `var issued_log: Array`, rempli seulement si `OS.has_feature("test")` ou si un drapeau de test est mis.
   - Il compare la liste obtenue à `game/tests/data/cb0_orders_golden.json`, généré sur le code actuel.
2. Déplacer `_unhandled_input`, `_finish_left`, `_finish_right`, `handle_group_key`, `_on_command`, `_next_formation`, `_available_selection`, l'état de glisser et de double clic, et `_deploy_selection` vers `BattleInput`.
   - Signaux : `command_requested(dict)`, `selection_changed(ids)`, `camera_focus_requested(point)`, `pause_toggled`, `speed_step(delta)`, `help_toggled`, `markers_toggled`, `screenshot_requested`.
   - La scène connecte ces signaux et garde `issue()`, le rendu et les appels au pont.
   - `smoke.gd` appelle `scene.handle_group_key` et `scene.issue` : on garde des délégations fines sur la scène.
3. Commit « refactor: extract battle input » une fois le test d'équivalence vert. Ce commit est séparé de la suite.
4. Sélection rapide, dans `battle_input.gd` :
   - Ctrl/Cmd+A sélectionne les unités du joueur présentes et hors déroute ;
   - un double clic gauche (350 ms, comme `DOUBLE_CLICK_MS`) sur une unité, au sol ou par la bannière, sélectionne le même `type` ;
   - le double clic sur une carte (`card_double_clicked`, qui aujourd'hui recentre seulement, `battle_scene.gd:1830`) sélectionne le même type **et** recentre.
   - Test ajouté à `cb0_input_equivalence_test.gd`.

### CB-M — Marqueurs, trajet, curseur (lot le plus lourd, quatre sous-lots en série)

**CB-M1 — Contours de formation (`game/`)**
- Nouveau `game/scripts/battle/battle_formation_outline.gd` :
  - un `Decal` par régiment ; texture de contour générée une fois (`Image` → `ImageTexture`), en trait plein et en pointillé ;
  - `size = (width, 30, depth)` pour couvrir le relief ; suit `x, z, facing` de `get_units()`.
- Fonction pure `outline_state(unit, selected, hovered, targeted) -> int` (Sélectionnée, Survolée, EnnemieSurvolée, EnnemieCiblée, Aucun), sur laquelle portent les tests. Le pulsé est obtenu par `modulate` animé dans `_process`.
- Supprimer `_rings` (création dans `_make_banner` 982-1019, mise à jour dans `_refresh_view` 1449-1476). `BattleMeshes.outline` reste s'il sert ailleurs.
- La « cible » est le champ `target` de l'unité sélectionnée.
- Test : `game/tests/cb_m1_outline_test.gd` (table d'états).
- Capture : `cbm_outline_shot.gd`.

**CB-M2 — Aperçu du trajet et curseur contextuel (`core/` + pont + `game/`)**

*Cœur* :
- `sim-battle/src/sim/pathing.rs` : extraire `plan_route` de `route()` (voir l'écart 1).
- Nouveau `sim-battle/src/preview.rs` : `BattleSim::preview_path(unit, x, z) -> Result<Vec<(f64,f64)>, PreviewError>`. La fonction prend `&self`, et le cache d'obstacles de siège, déterministe, est toléré.
- Nouveau `sim-battle/src/hover.rs` : `BattleSim::hover_context(x, z, selected, side) -> HoverContext { context: HoverKind, target: Option<UnitId>, piece: Option<usize>, compare: Option<Compare> }`.
  - Prise d'unité : point dans le rectangle `extent()` orienté par `facing`.
  - Prise de pièce de siège : `siege.rs`, `PieceKind`.
  - Portée : `effective_range_of` (`sim/scenario.rs:326`).
  - Ligne de vue : `visible()` et `fire_mode`.
  - Capacité de siège : `wall_breaker()`, bélier, échelles.
  - Zone de déploiement : `get_deployment_zone`.
  - La table des contextes suit exactement la spec.
- `Compare` : effectif, mêlée, défense, charge, tir et portée effective, moral, fatigue, bonus contre (`bonus_vs` du type). Les deux côtés sont au même format, avec des drapeaux d'avantage net pour chaque ligne. Le seuil d'avantage « net » vient de `data/rules/battle_hover.json`.

*Pont* :
- `battle_sim.rs` : `#[func] preview_path(unit_id, x, z) -> PackedVector3Array`, avec y tiré de `get_walk_height` ; tableau vide si aucun chemin.
- `#[func] hover_context(x, z, selected_ids) -> Dictionary`.

*Godot* :
- `battle_path_preview.gd` :
  - pointillés au sol et fantôme d'arrivée (réutilise le Decal de CB-M1, en semi-transparence) ;
  - recalcul si le curseur a bougé de plus de 5 m, au plus 10 fois par seconde ;
  - au-delà de 6 unités sélectionnées, un seul trajet depuis le barycentre ;
  - trajet rouge et `forbidden` si le chemin est vide ;
  - trajets, fantômes et flèche d'attaque persistent après l'ordre tant que l'unité reste sélectionnée : on lit `destination` et `target` dans `get_units`.
- `battle_cursor.gd` : correspondance `context` → icône, posée par `Input.set_custom_mouse_cursor(tex, shape, Vector2(16,16))`. Si le chemin est vide, `battle_input` n'envoie pas l'ordre.
- Icônes : curseurs `move`, `melee`, `ranged`, `ranged_blocked`, `siege`, `forbidden` (DA5, voir plus haut).

*Tests* :
- Cœur (`sim-battle/tests/cb_preview.rs`) :
  - le `preview_path` est identique à la suite des points que suit l'unité après l'ordre réel, en rase campagne avec gué et en siège ;
  - `state_digest` est inchangé après 100 appels de `preview_path` ;
  - `hover_context` : un cas par ligne de la table (portée, ligne de vue bloquée par un mur, porte, pièce de mur, déploiement, allié) ;
  - chiffres de `Compare`.
- Godot : test d'étranglement du recalcul (compteur d'appels), puis une capture.

**CB-M3 — Ordres en file (`core/` + `game/`)**

*Cœur* :
- `Command::Move` et `Command::Attack` gagnent `#[serde(default)] queue: bool`.
- `Unit` gagne `order_queue: VecDeque<QueuedOrder>`, avec 8 ordres au plus : `CommandError::QueueFull`, message en français.
- Quand l'ordre courant se termine (destination atteinte ou cible disparue), on dépile dans `sim.rs`. Tout ordre sans `queue` vide la file.
- Une déroute vide aussi la file.
- `get_units` expose `queue: Array[{x, z, facing?, target?}]`.
- `state_digest` inclut la file.

*Godot* :
- Maj + clic droit, et Maj + glisser-droit (avec largeur et orientation une fois CB1 en place), envoient `queue: true`.
- Points de passage numérotés, trajet segment par segment : `preview_path` depuis le dernier point de la file, grâce à un paramètre `from` optionnel ajouté à `preview_path`.
- File pleine : `forbidden` et infobulle.

*Tests* (`sim-battle/tests/cb_queue.rs`) : ajout, enchaînement, vidage, borne de 8, rejeu EP13 d'une partie avec file, relecture d'un rejeu antérieur à CB.

**CB-M4 — Portée au sol et comparaison au survol**
- `get_units` ajoute `effective_range` (`sim.rs:929`) et `fire_arc` : le demi-angle du champ de tir, à exposer s'il existe déjà, sinon une constante dans `data/rules/battle_hover.json`.
- Godot `battle_range_arc.gd` : arc au sol (mesh plaqué au relief, ou Decal en anneau) pour le tireur sélectionné ou survolé.
- Godot `battle_compare_panel.gd` :
  - ouvert si une seule unité est sélectionnée et qu'un ennemi est survolé, sur le terrain, par la bannière (`markers.hovered`) ou par la carte ;
  - il affiche `compare` avec des libellés RuleValues, en vert et rouge selon les drapeaux, sans pronostic.
- Test Godot : le panneau n'apparaît qu'avec une seule sélection, et les valeurs viennent du dictionnaire. Une capture.

### CB1 — Formation au glisser et verrouillage de groupe

*Cœur* :
- `Command::Move` gagne `#[serde(default)] width: Option<f64>` et `#[serde(default)] match_speed: bool`.
- `unit.rs` :
  - `line_files: Option<u32>`, avec des files calculées à partir de la largeur et de `spacing()` ;
  - les rangs sont bornés par type selon `data/rules/formation_width.json` (`min_ranks` par catégorie : piquiers ≥ 4, archers ≥ 2, etc., et `max_ranks`) ;
  - l'effectif est conservé, et `ranks_files` respecte `line_files` en Ligne.
- La largeur hors bornes est ramenée aux bornes. La largeur retenue est exposée (`width` existe déjà dans `get_units`).
- `match_speed` : dans `speed()` (`sim.rs:1351`), vitesse plafonnée à celle de l'unité la plus lente parmi les unités du même ordre groupé. On les identifie par un `group_tag` porté par l'ordre.
- `group_destinations` (`sim.rs:1184`) prend en charge des largeurs individuelles.
- Tests (`sim-battle/tests/cb1_width.rs`) : largeur vers rangs (bornes, effectif conservé, piquiers ≥ 4), absence de largeur = formation inchangée, `match_speed`, compatibilité du rejeu.

*Godot* (`battle_input.gd` et `battle_path_preview.gd`) :
- Pendant le glisser-droit, fantôme en direct :
  - une unité : la longueur du glisser donne la largeur ;
  - plusieurs unités : largeur répartie au prorata des effectifs, dans l'ordre gauche-droite courant (projection sur l'axe du glisser) ;
  - un clic simple ne transmet pas de `width`.
- Idem en déploiement (`DeploymentController.place`, `deployment_controller.gd:52`), qui passe la largeur à `deploy_unit`, avec l'argument ajouté au pont.
- Verrouillage (Ctrl/Cmd+G) : état dans `battle_groups.gd`.
  - Un groupe verrouillé traduit un ordre en ordres `Move` individuels : translation et rotation rigides des décalages relatifs, avec `match_speed` et le même `group_tag`.
  - Au glisser, le groupe est tourné et translaté, pas redistribué.
  - Une unité en déroute sort du groupe.
  - Cadenas sur les cartes (`unit_card.gd`).

### CB2 — Modes d'unité et icônes d'état (en parallèle avec CB3 et CB5)

- Ajout après relecture historique : mode permanent « Battre en brèche » pour les engins (effet sur les murs seulement, mangonneau moins efficace ; voir CB4).

*Cœur* :
- `Unit` gagne `mode_run: bool`, `guard: bool`, `skirmish: bool`, `melee_mode: bool`.
- Nouvelle commande `Command::SetMode { units, mode, enabled }`, additive pour le rejeu.
- Chiffres dans `data/rules/unit_modes.json` : multiplicateurs de course et de fatigue, distance de recul de l'escarmouche, seuil d'hésitation.
- Effets :
  - course persistante : `running` par défaut sur les `Move` ; le `run` ponctuel du double clic est conservé ;
  - garde : pas de poursuite (branche `sim.rs:1762`), pas d'entraînement, tient au contact ;
  - escarmouche : recul automatique face à une mêlée qui approche. Remplace l'usage de la capacité passive `Skirmish` à `sim.rs:2154` : les unités qui l'ont démarrent avec le mode actif, et le tir en marche reste lié au mode ;
  - mêlée : `can_shoot` est ignoré et l'unité engage.
- `get_units` expose `mode_run`, `guard`, `skirmish`, `melee_mode`, `charging`, `under_fire`, `engaged`, `wavering`. Ces états sont calculés dans le cœur ; les seuils viennent des données.
- IA (`ai.rs`, bloc `defensive` à `:1279`) : en posture défensive, tireurs légers en escarmouche et ligne en garde.
- Tests (`sim-battle/tests/cb2_modes.rs`) : effet de chaque mode, rejeu, IA défensive.
- Bataille de référence avant/après : `ep7_historical` et `eq7_cavalry` en release. Les marges doivent rester dans les bandes actuelles.

*Godot* :
- Touches F (tir à volonté), T (cycle de formation), R, G, K, M, remappées dans `battle_input.gd`.
- Mise à jour de `HELP_TEXT` et de `hotkey_labels()` dans `battle_hud.gd`, ainsi que des boutons `COMMANDS` (`battle_hud.gd:43`).
- Icônes de mode sur les cartes et dans la barre d'ordres.
- `BattleUnitMarkers.state_badges` (`:371`) : ajout des nouveaux états, au plus 3 pastilles, dans l'ordre déroute > hésitation > sous le feu > charge > mode.

### CB3 — Caméra et vue tactique (`game/` + un champ du cœur ; en parallèle, Sonnet pour la caméra)

- Vitesse :
  - `SPEEDS` passe à `[0.5, 1, 2, 4]` (`battle_scene.gd:48`, index par défaut sur ×1) ;
  - boutons de vitesse du HUD adaptés ;
  - `REPLAY_SPEEDS` inchangé ;
  - vérifier que le pas de simulation accepte ×0,5 (horloge EP8b, `ep8b_clock_test.gd`).
- `battle_camera.gd` :
  - bouton du milieu : glisser horizontal = lacet, vertical = tangage, avec des bornes de 5° à 85° ;
  - Maj + bouton du milieu = panoramique ;
  - drapeau `manual_pitch` qui suspend l'inclinaison automatique de `_apply` (`:155`) ; `look_at_point` le remet à zéro.
- Vue tactique, nouveau `battle_tactical_view.gd` :
  - Tab mémorise la caméra, puis passe en caméra du dessus cadrée sur le champ, avec un terrain assombri (`CanvasModulate` ou paramètre du shader de terrain) ;
  - les bannières sont forcées en pastilles B7 (`clustered`/icône), et les contours CB-M restent actifs ;
  - ennemis non `spotted` masqués (écart 2 : champ cœur `spotted` ajouté ici, avec un test cœur) ;
  - Tab ou Échap restaure la caméra ;
  - en déploiement, les contraintes de zone restent appliquées.
- Tests : `zg4_camera_test.gd` étendu (bornes, suspension et reprise), test de la vue tactique (entrée, sortie, caméra restaurée, ennemis non repérés masqués). Une capture.

### CB5 — Alertes de bataille (en parallèle, Sonnet)

*Cœur* :
- Nouveau `sim-battle/src/alerts.rs`, avec `enum AlertKind { Rout, GeneralDown, Flanked, Reinforcements, AmmoOut, WallBreached, GateDestroyed }`.
- Émission à côté des `log()` existants : déroute `sim.rs:2815`, général `:2660`, renforts `reinforcements.rs:102`, munitions `:2360`, murs et porte `:2463-2465`, `:1674`, `fire.rs:189`, `siege_assault.rs:482`. Pour le flanc, sur front montant de `flanked`.
- `take_new_alerts()` ; pont `#[func] get_alerts() -> Array[{kind, time, x, z, side, unit}]`.
- `data/rules/battle_alerts.json` : importance, durée (8 s), fenêtre de fusion (5 s), rayon de zone, ainsi que le schéma et le test associés.
- Tests (`sim-battle/tests/cb5_alerts.rs`) : chaque type émis une fois au bon endroit, déterminisme.

*Godot* :
- Nouveau `battle_alerts_column.gd` : colonne à gauche, 5 alertes au plus, icône et texte court, fusion.
- Un clic appelle `camera_rig.look_at_point`, avec un repère pulsé sur `battle_minimap.gd`.
- Cris via `BattleAudio.play_event` (`rout_cry` existe ; « général tombé » si un son AU1 existe, sinon son neutre).

### CB4 — Capacités actives (après CB2 ; session principale pour le cœur)

- **Pré-requis** : relecture historique de la liste (agent historien, sources dans `docs/research/cb4-capacites.md`), avant tout code.
- **Décisions après relecture historique (09-27, `docs/research/cb4-capacites.md`)** :
  - garder « Tir tendu » (archers seulement, précision et chevaux, portée −⅓ à −½, pas au contact ni sans munitions) ;
  - « Dresser les pavois » (arbalétriers, couleuvriniers) : facteur 0,35 conservé, plus un délai de pose et une protection de face seulement ;
  - « Charge en haie » remplacée par « Se rallier à la bannière » (chevaliers, immobiles, cohésion récupérée plus vite après le choc) ;
  - « Serrer les rangs » (ex-« Rangs serrés », branché sur `ShieldWall`) avec son coût : flancs plus exposés, pertes sous le tir, fatigue ;
  - « Hérisson » remplacé par « Piques plantées » (face seulement, immobile ; distinct du schiltron T) ;
  - « Tir de rupture » devient le mode permanent « Battre en brèche » (murs seulement, mangonneau moins efficace), déplacé dans CB2 (modes d'unité) ;
  - pieux : le passif actuel reste tel quel ; leur datation (attestés à partir de 1415) touche l'équilibrage de Crécy et Poitiers, hors du périmètre CB → suites de l'historien ;
  - écartés : flèches enflammées en rase campagne, duels, fuite simulée, pied à terre comme capacité d'unité.
- **Données** :
  - `data/battle_abilities/*.json`, avec un schéma et un test ;
  - champs : `id`, types d'unité, recharge, durée, modificateurs de stats et d'état, conditions (`stationary`, `not_engaged`, `ammo_gt_0`), règle d'IA ;
  - chargement par `data-model/src/load.rs` comme `battle_orders`, puis passage par `BattleSetup` (`battle_request.rs:553/823`, `historical_battles.rs:92`).
- **Cœur** :
  - nouveau `sim-battle/src/abilities.rs` : état par unité (recharge, temps restant) ; arrêt si une condition tombe, avec la raison exposée ;
  - `Command::UseAbility { units, ability }` ;
  - modificateurs appliqués dans `melee_damage`, `speed`, la portée, les dégâts de missile et les dégâts aux murs ;
  - le « pavois » passe de l'ordre de chef (`orders.rs:307`) à la capacité des arbalétriers, en gardant le facteur 0,35 ; `order_pavise.json` est retiré de `battle_orders` ;
  - `ShieldWall` est branché sur « Rangs serrés » ;
  - IA : une règle simple par capacité dans `ai.rs`.
- **Tests** (`sim-battle/tests/cb4_abilities.rs`) : recharge, conditions, arrêt, déterminisme du rejeu, IA. Référence EP7/EQ7 avant/après.
- **Godot** :
  - 1 à 3 boutons par carte (`unit_card.gd`), avec un cadran de recharge et une infobulle RuleValues ;
  - bouton grisé avec la raison si une condition n'est pas remplie ;
  - Alt/Option+1-4 agissent sur la sélection, en ne gardant que les unités qui ont la capacité ;
  - aide F1 mise à jour.

### CB6 — Formations de groupe (attaque, défense : placement proposé)

Demande du joueur (09-27) : choisir une formation d'attaque ou de défense pour un groupe et obtenir
automatiquement une proposition de placement des unités, en déploiement comme en bataille.

*Données* : `data/rules/group_formations.json` + schéma + test pytest. Chaque préréglage :
`id`, `name_fr`, `description_fr`, `stance` (`attack`/`defense`), et pour chaque rôle
(`infantry`, `foot_ranged`, `cavalry`, `siege`, `general`) une place : `row` (devant, ligne,
derrière, réserve), `lateral` (centre, ailes, réparti), écarts en mètres, largeur en files (via
CB1 `line_files`). Premier jeu, relu par l'historien avant la fusion :
- **Ligne de bataille** (défense) : fantassins au centre, tireurs devant, cavaliers sur les ailes,
  engins derrière — le placement par défaut actuel (`sim.rs`, `place_row`/`place_wings`), porté en données ;
- **Herse** (défense, à l'anglaise, Crécy/Azincourt) : hommes d'armes à pied au centre, archers en
  ailes avancées obliques, cavaliers en réserve derrière ;
- **Trois batailles** (attaque ou défense, à la française) : avant-garde, bataille, arrière-garde en
  trois lignes successives ;
- **Charge** (attaque) : cavalerie en première ligne, fantassins en soutien, tireurs sur les ailes ;
- **Colonne** (marche) : une file par ordre de vitesse.

*Cœur* : nouveau `sim-battle/src/group_formation.rs`, fonction pure
`BattleSim::formation_slots(preset, ids, x, z, facing) -> Vec<Slot{id, x, z, facing, width}>`
(rôles tirés de `category`/`mounted`, comme la mise en place initiale ; le général suit sa place ;
places ramenées dans la zone de déploiement en déploiement, hors eau profonde comme `dry_z`).
La mise en place initiale et `ai_deploy` passent par le préréglage « Ligne de bataille » : résultat
identique au placement actuel (test de non-régression : positions égales sur les scénarios de
référence, `b6` et `ep7_historical` inchangés). Aucun nouvel ordre : un préréglage produit des
`Move` individuels (avec `width`, `match_speed` et `group_tag` de CB1) ou des `deploy_unit`,
donc le rejeu EP13 n'est pas touché.

*Pont* : `formation_presets() -> Array[Dictionary]` et
`formation_slots(preset_id, ids, x, z, facing) -> Array[Dictionary]`.

*Godot* :
- barre de groupe (à côté du verrou de CB1) : sélecteur de préréglage, séparé en « Attaque » et
  « Défense », infobulle `description_fr` ; raccourci Alt+Maj+1…6 (Alt+1…4 est pris par les capacités de CB4 ; ajouté au remappage de CB2) ;
- en déploiement : bouton « Placer en formation » qui applique le préréglage à toute l'armée ou à la
  sélection, avec fantômes (décales CB-M1) avant validation ;
- en bataille : préréglage actif + clic droit (ou glisser-droit pour l'orientation) = fantômes des
  places puis ordres ; le groupe est verrouillé dans cette forme (CB1) ;
- aperçu des trajets par `preview_group` existant.

*Tests* : `sim-battle/tests/cb6_group_formation.rs` (rôles bien placés pour chaque préréglage,
effectif et ordre gauche-droite stables, zone de déploiement respectée, pas d'eau profonde,
« Ligne de bataille » = placement actuel, déterminisme) ; Godot `cb6_group_formation_test.gd` ;
capture `cb6_formation_shot.gd` (déploiement en Herse).

*Décisions après relecture historique (09-27, `docs/research/cb6-formations.md`, qui fournit les écarts par
rôle et les `description_fr`)* : six préréglages — Ligne de bataille, La herse, Trois batailles, Charge de
la chevalerie, Bataille à pied (ajout : masse démontée au centre, petite réserve montée sur un flanc,
Poitiers, Cocherel), Ordre de marche (ex-Colonne : ordre des batailles, vitesse du plus lent,
`stance: "march"` dans le schéma). « Ligne de bataille » garde les tireurs **derrière** l'infanterie
comme le placement actuel (non-régression de l'équilibrage) ; les tireurs devant sont l'affaire de la
herse. Raccourcis Alt+Maj+1…6. Le camp retranché (charrettes, palissade) est reporté à un lot futur.

*Ordonnancement* : après CB1 (largeur, verrou, `group_tag`), en vague 4 avec CB2, CB3 et CB5
(4 agents). Fusion : CB3, CB5, CB6, puis CB2 (qui intègre les raccourcis Alt+Maj+1…6 à l’aide F1).

## Ordre d'exécution

| Vague | Lots | Exécutant | Dépend de |
|---|---|---|---|
| 1 | CB0 | agent Sonnet (worktree) | — |
| 2 | CB-M1 → CB-M2 → CB-M3 → CB-M4 | session principale, ou un agent `cent-ans-dev` par sous-lot | CB0 ; CB-M2 attend la fusion de CV3-2 |
| 3 | CB1 | agent `cent-ans-dev` | CB-M |
| 4 | CB2, CB3, CB5, CB6 en parallèle (4 agents ≤ 6) | CB3 et CB5 : Sonnet ; CB2 et CB6 : `cent-ans-dev` (historien pour les préréglages CB6) | CB1 |
| 5 | CB4 (relecture historique, puis code) | historien, puis session principale | CB2 |
| fin | ADR 0095 finalisée, note `docs/archive/chantiers.md`, mémoire | session principale | — |

CB2, CB3 et CB5 modifient tous `battle_input.gd` et `battle_hud.gd`. On les fusionne (avec CB6) dans l'ordre CB3, CB5, CB6, puis CB2 : CB2 remappe les touches et réécrit l'aide en dernier.

## Vérification de bout en bout

- `cd core && cargo test`, avec les nouveaux tests `cb_preview`, `cb_queue`, `cb1_width`, `cb2_modes`, `cb4_abilities`, `cb5_alerts`, plus `ep13_replay` et `b6` (déterminisme).
- `cargo test --release -p sim-battle --test ep7_historical` et `--test eq7_cavalry`, avant et après CB2 et CB4 : les marges restent dans les bandes (Crécy et Azincourt 14-19/20, Poitiers 11-18/20, EQ7 ≥ 12/16).
- `uv run --project tools pytest` pour les schémas `formation_width`, `unit_modes`, `battle_hover`, `battle_alerts` et `battle_abilities`.
- `godot --headless --path game --script res://tests/smoke.gd`, `cb0_input_equivalence_test.gd` et les tests Godot de chaque lot.
- Captures `game/tests/cb*_shot.gd`, au plus 3 par lot, jugées par la session principale.
- Une partie pilote du joueur à la fin (une seule suffit).

## Fichiers critiques

- Godot :
  - `game/scripts/battle/battle_scene.gd`
  - `battle_input.gd` (nouveau)
  - `battle_camera.gd`, `battle_hud.gd`, `unit_card.gd`, `battle_unit_markers.gd`, `battle_groups.gd`, `deployment_controller.gd`, `battle_minimap.gd`
  - `game/scripts/audio/battle_audio.gd`
- Cœur, dans `core/crates/sim-battle/src/` :
  - `command.rs`, `sim.rs`, `sim/pathing.rs`, `unit.rs`, `ai.rs`, `orders.rs`, `replay.rs`
  - nouveaux : `preview.rs`, `hover.rs`, `alerts.rs`, `abilities.rs`
- Pont : `core/crates/godot-bridge/src/battle_sim.rs`, `data_store_rules.rs`.
- Données :
  - `data/rules/{formation_width,unit_modes,battle_hover,battle_alerts}.json`
  - `data/battle_abilities/`
  - `data/ui/icons_ink.json`
  - les schémas associés
