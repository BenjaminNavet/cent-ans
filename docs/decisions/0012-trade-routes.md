# ADR 0012 — Routes commerciales et accords : dérivées du graphe, pas d'état de rupture

Date : 2026-09-24. Statut : accepté. Lot C5 (`docs/design/2026-09-24-rapprochement-total-war.md`).

## Contexte

Le lot C5 demande des routes commerciales visibles (Bruges/laine, Bordeaux/vin, foires de Champagne en
déclin, étape de Calais...) et des accords commerciaux proposés/acceptés par la diplomatie existante
(`diplomacy.rs`), rompus par la guerre ou l'embargo (déjà unilatéral, `FactionState::embargoes`). La
refonte des colonies (fusionnée) donne un graphe de déplacement entre colonies (`movement_graph`,
`movement::edges`) et des ports (`Settlement::port`), assez pour dériver des routes sans nouvel état de
carte.

## Décision

- **Catalogue en données, revenu calculé.** `data/economy/trade.json` (schéma `trade.schema.json`) liste
  des comptoirs historiques (`hubs`, ancrés sur une colonie) et des routes entre deux comptoirs
  (`routes`, biens échangés, `base_value`, `declining` pour les foires de Champagne). Le chemin réel
  (terrestre le long du graphe, maritime entre ports) est calculé à chaque requête par un Dijkstra propre
  sur `movement_graph` (`sim-campaign/src/trade.rs::shortest_path`), pas stocké : la géométrie du graphe
  ne change pas en cours de partie, seule la sécurité de la route varie.
- **Aucun état « route coupée » à synchroniser.** `trade_routes()` est une requête pure recalculée
  chaque saison à partir de l'état courant (contrôleurs des comptoirs, sièges, armées présentes, guerre,
  embargo, accord). Une route se coupe et se rétablit d'elle-même selon l'état du tour, comme
  `embargo_income_factor` le fait déjà pour les embargos — pas de transition à intercepter.
- **Accord commercial = `Proposal::TradeAgreement`, jamais effacé par la guerre.** Réutilise le flux
  d'offres existant (comme l'alliance). `FactionState::trade_agreements` (miroir bilatéral,
  `#[serde(default)]`) reste en place pendant une guerre ou un embargo : `has_trade_agreement` répond
  toujours `true`, mais `trade_routes()` ignore son bonus (+30 %, `AGREEMENT_BONUS_PERCENT`) tant que
  guerre ou embargo dure, et coupe la route sous-jacente comme n'importe quelle route sans accord. Cela
  évite un ordre de rupture automatique à émettre au bon moment (déclaration de guerre, embargo posé par
  un tiers...) et republier à la paix : l'accord « dort » puis reprend seul.
- **Revenu = distance × sécurité × déclin × monnaie, partagé aux deux bouts.** `base_value` est la valeur
  de référence ; `distance_factor` (coût du chemin) et `THREAT_SECURITY_FACTOR` (siège = coupure nette,
  armée ennemie sur le chemin = sécurité divisée par deux par nœud menacé) sont de simples facteurs
  multiplicatifs, pas de nouvelle simulation. Le facteur de monnaie (H5, `coinage_factor`) relie C5 à H5
  sans dupliquer sa logique : lecture seule de `price_level`.
- **Pas de blocus naval dédié.** Le jeu n'a pas de bataille navale (choix v1 confirmé) ; un port assiégé
  ou une armée ennemie stationnée dessus suffit à représenter un blocus au sens du cahier des charges.

## Conséquences

- `trade.rs` est un module autonome appelé une fois par tour (`turn.rs`, après
  `economy::resolve_economy`) : pas de couplage avec `buildings.rs`/`movement.rs` au-delà de lectures
  publiques déjà existantes (limite les conflits avec C2/C4 en parallèle).
- Le catalogue de routes est fixe (comptoirs/paires codés en données) plutôt que dérivé dynamiquement de
  toute paire de colonies avec port : plus simple à équilibrer et à documenter historiquement, au prix
  d'une extension manuelle si de nouveaux comptoirs doivent apparaître.
- Aucun changement de `STATE_VERSION` : `trade_agreements` et `trade_income_last_turn` sont
  `#[serde(default)]`, une ancienne sauvegarde se charge sans accord commercial actif.
