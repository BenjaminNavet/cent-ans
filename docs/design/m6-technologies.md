# M6 — Technologies : spécification

Date : 2026-09-23. Objectif : deux arbres de technologies (militaire, civil) financés par des points de
recherche, qui débloquent unités, bâtiments et effets. Les données existent déjà
(`data/technologies/*.json`, 22 techs, champs `branch`, `tier`, `cost`, `prerequisites`, `unlocks{units,
buildings}`, `effects`, `historical_year`) ; `FactionState::technologies` contient déjà les techs de départ.

## 1. Données (`data/`)
- Compléter jusqu'à ≈ 30 techs (≈ 15 par branche, tiers 1-4) en restant historique : ajouter par exemple
  `tech_pavise`, `tech_plate_harness_tournament` → `tech_brigandine`, `tech_handgonnes`, `tech_hand_cannon_drill`,
  `tech_compagnies_d_ordonnance` (1445), `tech_francs_archers` (1448), `tech_double_entry` (comptabilité en
  partie double), `tech_letters_of_credit`, `tech_quarantine` (Raguse 1377), `tech_hanseatic_trade`,
  `tech_gothic_flamboyant`… Chaque tech garde `historical_year` et `sources`.
- `EffectKind` nouveau si besoin : `ResearchPoints` (bâtiments : université, monastère, guilde → points).
  Ajouter l'effet aux bâtiments existants concernés (`data/buildings`), sinon créer `bld_university`,
  `bld_scriptorium`, `bld_guild_hall` s'ils manquent.
- Mettre à jour `data/schemas/` et `core/crates/data-model` (tests `real_data.rs`).

## 2. Simulation (`core/crates/sim-campaign`, nouveau `research.rs`)
- `FactionState` gagne `research: Option<TechnologyId>` (en cours), `research_progress: u32`,
  `research_points_last_turn: u32`. `state_version` +1 (coordonner : prendre la valeur suivante de
  `STATE_VERSION` au moment du merge).
- Points par tour = base 5 + somme des effets `ResearchPoints` des provinces possédées + bonus
  (gouvernance du dirigeant / 2, arrondi). Payés en fin de tour (phase économie).
- Ordre `research { technology }` : tech connue, non acquise, prérequis acquis. Changer de recherche
  conserve la progression de la tech abandonnée (`research_banked: BTreeMap<TechnologyId, u32>`).
- Achèvement : tech ajoutée à `technologies`, événement `technology_researched` (français), recherche
  vidée. Coût effectif = `cost` × (1 + 0,25 si `historical_year` > année courante + 20) : les techs trop en
  avance coûtent plus cher.
- Effets : `recruitable` refuse les unités dont la tech débloquante n'est pas acquise (vérifier
  l'existant) ; `buildable` idem pour les bâtiments ; les `effects` des techs s'agrègent en
  `faction_tech_effects(faction) -> EffectTotals` appliqués au revenu (`TaxIncome`, `TradeIncome`), à la
  population (`Health`, `Growth`, `Unrest`) et aux batailles (`ArmyMorale`, `ArmyRanged`, `ArmyMelee`,
  `ArmyArmor` par `unit_category` → brancher dans `battle_auto::Side` comme les bonus de général).
- IA minimale : choisit la tech la moins chère disponible, en alternant les branches.
- Tests (≥ 8) : points par tour, ordre refusé (prérequis, déjà acquise), achèvement et événement,
  progression conservée en changeant, déblocage d'unité, effet de tech sur une bataille, coût anachronique,
  sauvegarde round-trip, déterminisme.

## 3. API GDExtension (`CampaignSim`)
- `get_tech_tree(faction) -> [{id, name, branch, tier, cost, effective_cost, prerequisites[], unlocks{units[],
  buildings[]}, effects[{kind, value, mode}], description, historical_year, state: "known"|"available"|"locked"|
  "researching", progress}]`.
- `get_research(faction) -> {technology, name, progress, cost, points_per_turn, turns_left}` (vide si aucune).
- Ordre `research { technology }` via `submit_order`.

## 4. Interface Godot
- Panneau « Technologies » (bouton dans la barre, touche T) : deux colonnes/onglets Militaire et Civil,
  nœuds par tier reliés par des lignes de prérequis, états colorés, info-bulle (effets, déblocages, année
  historique), clic = `research`. Barre de progression de la recherche en cours dans le HUD.
- Journal : couleur dédiée `technology_researched`.
- Smoke test : lancer une recherche, 20 tours, au moins une tech acquise.
- Capture : `docs/img/godot-tech-tree.png` (`--stage=tech`).

## 5. Critères de fin
Tests Rust verts, smoke vert, capture relue, `docs/status.md`, `docs/roadmap.md`, `docs/godot-map.md`,
`docs/design/data-model.md` à jour.
