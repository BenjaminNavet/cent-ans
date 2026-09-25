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
- **Accord commercial = l'article de traité de DP1 (révisé le 25/09, voir « Unification avec DP1 »).**
  La première version (`Proposal::TradeAgreement`, champ `FactionState::trade_agreements` qui « dormait »
  pendant une guerre) est abandonnée à la fusion de DP1.
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
- Aucun changement de `STATE_VERSION` : `trade_income_last_turn` est `#[serde(default)]` ; les accords
  vivent dans le registre DP1 (déjà `#[serde(default)]`).

## Unification avec DP1 (25/09, lot C5R)

DP1 (ADR 0025) a apporté, pendant que C5 attendait sa fusion, la diplomatie multi-clauses
(`Proposal::Treaty`, `negotiation.rs`) avec un article `Article::TradeAgreement` autonome : registre
`DiplomaticLedger::trade_agreements` et bonus forfaitaire de +2 % de revenu par accord (+8 % au plus).
Deux représentations du même accord coexistaient donc. Choix retenu :

- **Une seule représentation dans l'état : le registre DP1** (`FactionState::ledger.trade_agreements`,
  miroir bilatéral). `CampaignState::has_trade_agreement` le lit ; `trade_routes()` y trouve le bonus
  de +30 % (`AGREEMENT_BONUS_PERCENT`) des routes entre les deux signataires. Supprimés :
  `Proposal::TradeAgreement`, `Order::ProposeTradeAgreement`, `FactionState::trade_agreements`, l'IA
  `plan_trade_agreements` de C5 et le facteur forfaitaire `negotiation::trade_income_factor`.
- **Pourquoi l'article de traité plutôt que la proposition de C5.** Le traité s'intègre mieux : il se
  combine aux autres clauses (paix + accord commercial, mariage + accord), a déjà une évaluation
  détaillée (richesse du partenaire, rival, embargo), une IA qui le propose (`ai::diplomacy_eval`),
  l'écran de négociation, l'historique des traités et la confiance (« standing »). La proposition de C5
  n'apportait qu'un flux d'offre parallèle. On garde de C5 le seul ordre sans équivalent DP1 :
  `Order::BreakTradeAgreement` (rupture unilatérale, qui retire l'accord du registre).
- **Revenu : les routes, plus de forfait.** L'accord ne rapporte que par les routes communes (+30 % de
  leur valeur) ; l'évaluation de l'article compte « Routes commerciales communes » (+4 par route, +12 au
  plus, `trade::common_routes`, lecture des seuls contrôleurs de comptoirs, sans Dijkstra). Un accord sans
  route commune garde sa valeur diplomatique (confiance DP1) mais ne rapporte rien.
- **Guerre et embargo.** La règle de DP1 prévaut : la guerre efface l'accord du registre (comme l'accès
  militaire et les tributs). L'embargo ne l'efface pas : il coupe les routes (`cut_reason = "embargo"`)
  et l'accord est affiché « suspendu » dans le panneau de diplomatie.
- **Budget.** Le revenu commercial (`trade_income_last_turn`, crédité après l'impôt) entre dans la ligne
  « Recettes » de l'historique du budget (U3) et dans `net_income`/`faction_net_last_turn`.

