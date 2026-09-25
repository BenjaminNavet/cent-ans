# C5 — Routes commerciales et accords (rapprochement Total War)

## État (24/09, démarrage)
Squelette data-model posé : `data/economy/trade.json` (hubs + routes), chargé
dans `GameData::trade` (`core/crates/data-model/src/entities/trade.rs`,
`load.rs`). Compile.

## État (24/09, suite)
- Rust fait et testé (`core/crates/sim-campaign/tests/c5_trade.rs`, 8 tests
  verts : revenu, coupure guerre/embargo/siège, bonus d'accord, déterminisme,
  ancienne sauvegarde) : `data/economy/trade.json` + schéma,
  `sim-campaign/src/trade.rs` (routes dérivées d'un Dijkstra propre sur
  `movement_graph`, `Proposal::TradeAgreement` dans `diplomacy.rs`,
  `Order::ProposeTradeAgreement`/`BreakTradeAgreement`, appelé depuis
  `turn.rs` après `economy::resolve_economy`). `campaign.rs` (test existant
  de revenu de France) corrigé pour inclure `trade_income_last_turn`.
- Bridge Godot : `campaign_sim_trade.rs` (`get_trade_routes`),
  `get_faction_economy`/`get_diplomacy` étendus (`trade_income`,
  `trade_agreement`), `evaluate_proposal` gère `ProposeTradeAgreement`.
- UI : `trade_route_layer.gd` (ruban `PolylineMesh`, épaisseur = valeur,
  brouillard C1), bouton « Commerce » + touche R (`map_toggle_trade`),
  infobulle au survol (`campaign_map._update_trade_hover`), section
  Commerce du panneau de diplomatie (accords, routes, revenu), ligne
  Commerce du panneau de faction (revenu, détail en infobulle), rubrique
  « Commerce » du rapport de saison et du journal.
- Smoke : `_run_trade()` ajouté (routes, accord proposé/rompu, coupure par
  embargo, revenu dans `get_faction_economy`, panneaux réels) ; stage
  `--stage=trade` pour la capture d'écran.

## État (25/09, reprise après crash)
- ADR renuméroté `docs/decisions/0012-trade-routes.md` (0010 = mouvement
  libre sur main, 0011 réservé à C4).
- Corrigés : boucle infinie bouton « Commerce » ↔ bascule
  (`set_pressed_no_signal`), appel `get_trade_routes` sur le mock (garde
  `has_method`), smoke du panneau de faction (instancier la scène).
- Capture : `docs/img/c5-commerce/routes.png` (`--stage=trade`).
- Smoke : 24 « smoke OK » dont « trade ».

## État (25/09, C5R : reprise C4 + C5, fusion main, unification DP1)
- Branche `worktree-agent-ae7dec5cc632f4a8c` = `integration/tw` (C4 + C5 déjà réunis, 6ebe9658)
  + `main` (M2-M4, G1, U1/UI2, DP1, CV1/V4/CM2, Q1). Conflits résolus : `load.rs`, `state.rs`,
  `setup_1337.rs`, `turn.rs` (édits C4 + commerce C5 dans `resolve_end_of_turn`), `lib.rs`,
  `economy_balance.rs` (revenu commercial dans les recettes), `diplomacy.rs`, pont
  (`campaign_sim_diplomacy.rs`, `lib.rs`), `campaign_map.gd` (étapes trade + movement),
  `season_report.gd` (rubriques de main ; `trade` → Trésor, `edict` → Vos terres ; style « Commerce »),
  `diplomacy_panel.gd` (écran plein DP1 + routes communes, rupture d'accord).
- Unification DP1 (ADR 0012 § « Unification avec DP1 ») : l'article de traité `trade_agreement`
  est l'accord de C5 ; `Proposal::TradeAgreement`, `Order::ProposeTradeAgreement`,
  `FactionState::trade_agreements`, `plan_trade_agreements` et `trade_income_factor` supprimés.
  Test `c5_trade::trade_article_counts_common_routes_in_the_evaluation` + accord signé par
  `apply_treaty` dans les tests C5. Smoke : l'accord passe par `propose_treaty`.
- `cargo test` : 580 verts.

## Pistes
- Icônes de marchandise et navires/charrettes animés sur les routes (seul le
  ruban est fait) ; entrée d'encyclopédie dédiée.
- Accord actif non distingué visuellement (un matériau par maillage) :
  troisième instance ou shader à couleur par sommet.
- L'infobulle de route ne s'efface pas d'elle-même : le survol de province
  la remplace.
- `trade.rs::shortest_path` fait un Dijkstra sur `movement_graph` (`movement::edges` existe
  toujours après M2-M4).

## Décisions
- Un seul fichier `data/economy/trade.json` (hubs + routes) plutôt que deux
  dossiers : moins de code de chargement pour un catalogue de taille fixe.
- (Remplacé le 25/09) Accord commercial = article de traité DP1, stocké une
  seule fois dans le registre DP1 ; la guerre l'efface, l'embargo le suspend.

## Prochaine étape
Sondes d'équilibre (balance_probe 8×200, century_probe 5×464) avant/après, puis build GDExtension,
import, smoke, captures de la couche commerciale et du panneau d'édits.
