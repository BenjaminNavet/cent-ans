# C5 — Routes commerciales et accords (rapprochement Total War)

## État (24/09, démarrage)
Squelette data-model posé : `data/economy/trade.json` (hubs + routes), chargé
dans `GameData::trade` (`core/crates/data-model/src/entities/trade.rs`,
`load.rs`). Compile.

## Prochaine étape
- Écrire `data/economy/trade.json` (hubs historiques : Bruges, Calais,
  Londres, Bordeaux, La Rochelle, Southampton, Anvers, Troyes/Provins,
  Gênes, Venise) + `data/schemas/trade.schema.json`.
- `core/crates/sim-campaign/src/trade.rs` : accords (Proposal::TradeAgreement
  côté `diplomacy.rs`), routes dérivées (chemin sur `movement_graph`,
  sécurité, embargo/guerre, coinage), phase de résolution appelée depuis
  `turn.rs` après `economy::resolve_economy`.
- Bridge Godot (`campaign_sim_trade.rs`) + UI (couche carte, onglet
  Commerce, rapport de saison), smoke « trade », captures
  `docs/img/c5-commerce/`.

## Décisions
- Un seul fichier `data/economy/trade.json` (hubs + routes) plutôt que deux
  dossiers : moins de code de chargement pour un catalogue de taille fixe.
- Accord commercial = `Proposal::TradeAgreement` réutilisant le flux
  d'offres existant (comme l'alliance) ; rompu automatiquement (pas d'état à
  synchroniser) dès que guerre ou embargo entre les deux parties : le calcul
  du revenu vérifie l'état courant à chaque saison plutôt que d'intercepter
  les événements qui le rompent.
