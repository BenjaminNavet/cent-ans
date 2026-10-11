# 0344 — Vocations des colonies

Statut : accepté (lot RG10 du chantier RG, retours de gestion du joueur).

## Contexte
Depuis CO (ADR 0291), une province compte au plus 5 colonies, à 6/4/3/3/2 emplacements de bâtiments (cité, ville,
château, abbaye, village). Le joueur veut spécialiser ses colonies : choisir ce qu'elles sont, plutôt que de
construire au gré du rendement.

## Décision
- **Données** : `data/settlements/vocations.json` (schéma `data/schemas/vocations.schema.json`, chargé par
  `settlement_load.rs` à part des fichiers `prov_*.json` et de `rules.json`). Six vocations alignées sur les catégories
  et bâtiments existants : `agricultural` (fours, greniers, moulins, pressoirs, adduction d'eau), `mercantile`
  (catégorie commerce), `military` (catégorie military), `religious` (religious + sanitary), `artisanal` (forge,
  fonderie d'étain, tissage, atelier de siège), `fortified` (catégorie fortification).
- **Chaque vocation** : catégories/bâtiments concernés, `effect_bonus_percent` (+15 %), `cost_reduction_percent`
  (−20 %), `off_vocation_surcharge_percent` (+10 %), paliers de cohérence cumulatifs (`min_buildings` 2 puis 4 ; 2 puis 3 pour
  la fortifiée, 3 emplacements seulement) portant des `effects` ordinaires. `bonus_excluded_effects` (niveau de fortification,
  emplacements de recrutement) n'est pas multiplié.
- **État** : `SettlementState.vocation: Option<String>` et `vocation_set_turn`, `#[serde(default)]` : sans vocation, aucun
  effet et les anciennes sauvegardes se chargent.
- **Effets = delta** (`sim-campaign/src/vocations.rs`) : la vocation ne modifie pas la liste de bâtiments ; `delta_effects`
  calcule la part de bonus des effets des bâtiments concernés plus les paliers atteints, et la somme est ajoutée à côté des effets
  de bâtiments dans `settlement_effects`, `province_building_effects` (même pondération `province_effect_percent`),
  `construction_speed_percent`, l'impôt par colonie (`settlement_tax_with`, `income_breakdown`) et le prestige/la piété
  annuels. Les prévisions RG8 (`forecast.rs`) réutilisant ces calculs le prennent donc en compte.
- **Coût de construction** : `adjusted_cost` dans `buildable_with_supply` (remise sur les bâtiments concernés, surcoût sur
  les autres) ; l'ordre `Build`, l'IA et les listes d'options lisent le même prix.
- **Ordre** `Order::SetSettlementVocation { settlement, vocation }` : colonie possédée et non ruinée, vocation connue et
  différente de l'actuelle, trésor suffisant ; coût `adoption_cost[type de colonie]` (la toute première vocation ne paie que
  `first_adoption_percent`), puis verrou `change_lock_turns` tours (saisons). Pas d'ordre pour retirer la vocation.
- **IA** (`CampaignState::suggest_vocation`, appelé par `ai::campaign::economy::plan_vocations`) : vocation qui concerne le
  plus de bâtiments existants (ou en construction), ex æquo à la première de la liste ; adoption à partir de
  `ai_min_buildings` (2) ; changement seulement si une autre vocation devance de `ai_switch_margin` (3) bâtiments et que le
  verrou est levé ; payé sur le magot (au-delà de la réserve), une adoption par colonie et par tour.
- **Getters** : `CampaignState::settlement_vocation` (vocation, décompte, palier suivant, effets chiffrés) et
  `vocation_options` (coût, aperçu, adéquation), exposés par le pont `get_settlement_vocation` / `get_vocation_options` et
  `SimFacade`.

## Conséquences
- Équilibrage : bonus modéré (+15 % sur des effets de +5 à +20, soit 1 à 3 points, et un palier 2 de l'ordre d'un bâtiment de
  niveau 1). L'IA paie comme le joueur, n'adopte qu'avec deux bâtiments concordants et change très rarement ; le gain moyen
  est donc de l'ordre de quelques pour cent de revenu sur les colonies spécialisées. À surveiller : `campaign_probe` (bandes
  de revenus et de révoltes).
- Une vocation peut être retirée de la liste de données sans casser une sauvegarde : une vocation inconnue vaut « sans vocation ».
