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

## Prochaine étape
- Vérifier après `core/build.sh` + `--import` + smoke : compter les
  « smoke OK » (24 attendues, 23 + trade) et l'absence de « SCRIPT ERROR ».
- Capture `docs/img/c5-commerce/` via
  `--screenshot=docs/img/c5-commerce/trade.png --stage=trade`.
- Piste non faite (budget) : petites icônes de marchandise / navires-charrettes
  animés sur les routes (juste le ruban parchemin pour l'instant) ; entrée
  encyclopédie dédiée.

## Décisions
- Un seul fichier `data/economy/trade.json` (hubs + routes) plutôt que deux
  dossiers : moins de code de chargement pour un catalogue de taille fixe.
- Accord commercial = `Proposal::TradeAgreement` réutilisant le flux
  d'offres existant (comme l'alliance) ; rompu automatiquement (pas d'état à
  synchroniser) dès que guerre ou embargo entre les deux parties : le calcul
  du revenu vérifie l'état courant à chaque saison plutôt que d'intercepter
  les événements qui le rompent.
