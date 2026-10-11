# 0345 — Panneau Bâtiments avec arbre de construction (RG11)

## Contexte
L'onglet « Bâtiments » d'une colonie (CO-C, ADR 0292) montrait des cartes d'emplacements avec des
boutons « Améliorer » : le joueur le trouvait « identique à avant » et ne voyait ni ce qu'il pouvait
bâtir plus tard, ni pourquoi un bâtiment lui était refusé. Les raisons de refus existaient dans le
cœur (`CampaignState::buildable`, `build_blocker`) mais en une seule phrase par bâtiment, mêlées aux
refus passagers (trésor, file pleine, emplacements pleins), et sans structure pour une interface.

## Décision
**Cœur.** Nouveau module `sim-campaign/src/building_tree.rs` et getter `CampaignState::building_tree`
(pont : `CampaignSim.get_building_tree(settlement_id)`, façade : `SimFacade.building_tree`). Lecture
seule, aucune règle ajoutée :
- un nœud par bâtiment de `data/buildings/`, avec chaîne (`parent`, `root`, `depth`), état
  (`built` / `building` / `queued` / `available` / `locked`), `can_build`, coût, durée, entretien,
  `takes_slot`, tours restants, rang de file et liste de prérequis `{kind, id, text, met}` ;
- les verrous viennent de `structural_blockers`, extrait de `build_blocker` sans changer son ordre
  ni ses textes (`buildable` reste identique) : chaîne, bâtiment requis, technologie, ressource,
  côte, rivière, unicité ; le rang (`settlement_kinds`) s'y ajoute côté arbre ;
- un refus passager (trésor, file pleine, emplacements pleins) ne verrouille pas : le nœud reste
  `available`, `can_build` est faux et `blocked_now` donne la raison de `buildable` ;
- raisons en clair, produites par le cœur : « Requiert la technologie X », « Requiert le rang Cité
  ou Ville », « Requiert Marché (en chantier) », « Requiert la ressource X dans la province »…
- `slots` (`used/max`), `queue_size` et `queue` (chantier en cours puis file, avec durée totale).
Aucune donnée ne manquait : tous les prérequis (`upgrades_from`, `required_building`,
`required_technology`, `required_resource`, `requires_coastal`, `requires_river`, `unique_per_faction`,
`settlement_kinds`) sont déjà dans `data/buildings/` et le schéma ; aucun prérequis implicite dans le
code. Le schéma n'est donc pas modifié.

**Interface.** `settlement_buildings_view.gd` est refait : en-tête de palier (inchangé), emplacement
vide `VocationSlot` réservé à RG10, rangée de cases (bâtiment + niveau, chantier + barre + tours
restants, case libre), file d'attente à 3 places, puis l'arbre : une section par catégorie, une
colonne par chaîne d'amélioration, un bouton par nœud (couleur et texte selon l'état). Un clic sur un
nœud constructible émet `build_requested` → ordre `build` existant. L'infobulle riche existante
(`RichTooltip.building_spec`) reçoit une ligne `live` tirée du nœud : coût, durée, entretien, effets,
prérequis (états du cœur), rang requis, emplacement pris, raison. Les bâtiments d'autres rangs sont
masqués par défaut (case « Autres rangs »).

## Conséquences
- Le GDScript ne recalcule aucune règle : états, raisons, coûts viennent du cœur.
- L'ancienne section « Liste détaillée et construction » et la boîte « Chantier » (annulations)
  restent sous la vue ; elles pourront disparaître quand l'arbre aura fait ses preuves.
- Les vocations (RG10) s'inséreront dans `VocationSlot` sans toucher au reste.
- L'arbre porte tous les bâtiments (≈ 40) : lisible grâce au filtre de rang par défaut.
