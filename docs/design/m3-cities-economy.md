# M3 — Villes et économie : spécification

Date : 2026-09-23. Objectif : donner vie aux provinces. La population par classe évolue, les bâtiments se
construisent, quatre jauges par classe (mécontentement, santé, richesse, satisfaction en biens) réagissent
aux décisions du joueur et influencent revenus, recrutement et ordre public. Le taux d'imposition devient
un levier. Tout vit dans `sim-campaign` ; Godot affiche.

## 1. Modèle (`core/crates/sim-campaign`)

### 1.1 Population par classe (`PopulationClasses`, déjà dans l'état, devient dynamique)
Pour chaque classe (paysans, bourgeois, clergé, noblesse) : `count`, `unrest`, `health`, `wealth`,
`goods_satisfaction` (0-100). Par saison, dans `population.rs` :
- Croissance : `count *= 1 + base_growth × f(health) × g(devastation)` avec base 0,4 %/saison paysans,
  0,3 % bourgeois, 0,05 % clergé, 0,1 % noblesse ; `f(health) = (health - 50) / 50` (santé < 50 → déclin) ;
  dévastation > 50 → croissance nulle. Recrutement retire les soldats de la classe source (déjà partiel).
- Santé : tend vers `50 + effets bâtiments sanitaires (Health) + (goods_satisfaction - 50) / 4 - surpopulation`
  (surpopulation = max(0, count / capacité_province - 1) × 20, capacité = 40 000 × (1 + niveaux de bâtiments
  de production)). Vitesse : 20 % de l'écart par saison.
- Richesse : tend vers `base_classe + effets (Wealth, TradeIncome) - taux d'imposition × 30 - dévastation / 2`
  ; base 30 paysans, 55 bourgeois, 50 clergé, 70 noblesse. Vitesse 15 %/saison.
- Satisfaction en biens : dépend des ressources accessibles à la faction (§ 1.3) et des bâtiments de
  commerce (GoodsSatisfaction) : `40 + 10 × nb_catégories_de_biens_disponibles (max 5) + effets`, vitesse 25 %.
- Mécontentement : tend vers `taux d'imposition × 40 + dévastation / 2 + (50 - goods_satisfaction) / 3
  + (50 - health) / 4 + occupation étrangère (+25 si contrôleur ≠ propriétaire) + religion différente (+10)
  - garnison (min(20, effectif / 100)) - effets Unrest (négatifs = apaisement)`, borné 0-100, vitesse 20 %.
- Révolte : si mécontentement moyen pondéré > 75 pendant 2 saisons consécutives → événement `revolt`,
  garnison réduite de 25 %, revenus de la province nuls ce tour ; > 90 → la province passe à un
  contrôleur « rebelles » (`fac_rebels`, faction virtuelle non jouable ajoutée à `data/factions/`).

### 1.2 Bâtiments (`buildings.rs`)
- `ProvinceState.buildings: Vec<BuildingId>` (initialisé depuis `data/provinces/*.json` `buildings`) et
  `construction: Option<Construction { building, turns_left }>` (une construction à la fois par province).
- Ordre `build { province, building }` : requiert contrôle + propriété, prérequis (`required_building`,
  `required_technology` dans `FactionState.technologies`, `required_resource` dans la province, côte/rivière),
  `upgrades_from` (remplace l'ancien), pas déjà présent, trésor ≥ coût ; coût prélevé immédiatement.
  `cancel_build { province }` rembourse 50 %.
- Effets appliqués via une fonction unique `province_effects(&self, data, province) -> EffectTotals`
  qui somme les `Effect` (mode `flat`/`percent`) de tous les bâtiments : `TaxIncome`, `TradeIncome`, `Health`,
  `Unrest`, `Wealth`, `GoodsSatisfaction`, `Growth`, `Garrison` (unités de garnison gratuites), `FortificationLevel`
  (remplace la constante de siège), `RecruitCost`, `Supply`. Les autres `EffectKind` sont ignorés pour l'instant.
- Entretien des bâtiments (`upkeep`) ajouté à la dépense de faction.
- `buildable(&self, data, province) -> Vec<BuildOption { building, name, cost, turns, available, reason }>`.

### 1.3 Ressources et biens
- Chaque province produit ses `resources` (data). La faction dispose de l'ensemble des ressources des
  provinces qu'elle contrôle + celles de ses alliés (commerce formel en M5). Catégories de biens :
  nourriture, textile, matériaux, métal, luxe (champ `category` de `data/resources`).
- `FactionState.goods: BTreeMap<ResourceId, u32>` recalculé chaque tour (nombre de provinces productrices).

### 1.4 Impôts
- `FactionState.tax_rate: TaxRate { Low, Normal, High }` (×0,7 / ×1,0 / ×1,4 sur le revenu fiscal ;
  mécontentement selon § 1.1). Ordre `set_tax_rate { rate }`.
- Revenu par province = `Σ_classe count × taux_classe × wealth / 50` × `(1 - devastation/100)` × `tax_rate`
  × `(1 + TaxIncome %)` + `TradeIncome` flat/percent sur la part bourgeoise ; garde `TAX_EFFICIENCY`.
- `faction_summary` gagne `projected_income` (calculé sur l'état courant) et `upkeep` détaillé
  (`army_upkeep`, `building_upkeep`).

### 1.5 Événements
`revolt`, `building_completed`, `plague` (déclenché si santé moyenne < 30 : -10 % population, +20 mécontentement),
`famine` (dévastation > 70 en hiver : -5 % paysans). Textes français.

### 1.6 Tests
Croissance positive/négative selon santé ; construction : prérequis, coût, durée, effet appliqué au tour
suivant ; taxe haute → revenu +40 % et mécontentement en hausse ; révolte après mécontentement soutenu ;
biens : plus de catégories → satisfaction plus haute ; déterminisme 20 tours conservé ; sauvegarde round-trip
avec les nouveaux champs (`state_version` → 2, `load_json` refuse la version 1 avec un message clair).

## 2. API GDExtension (`CampaignSim`)
- `get_province_city(id) -> Dictionary { classes: { peasants: {count, unrest, health, wealth, goods_satisfaction}, ... },
  buildings: [{id, name, category, upkeep}], construction: {building, name, turns_left}?, fortification_level,
  capacity, buildable: [{building, name, category, cost, turns, available, reason}], resources: [ids],
  effects: {tax_income, trade_income, health, unrest, ...} }`.
- `get_faction_economy(id) -> { treasury, income, projected_income, army_upkeep, building_upkeep, tax_rate,
  goods: {resource_id: count}, goods_categories: [..] }`.
- Ordres via `submit_order` : `{"type":"build","province":..,"building":..}`, `{"type":"cancel_build","province":..}`,
  `{"type":"set_tax_rate","rate":"low|normal|high"}`.

## 3. Interface Godot
- Panneau province : onglets « Garnison » (existant) et « Ville » : quatre lignes de classes avec effectif et
  quatre mini-jauges colorées (vert → rouge), liste des bâtiments avec entretien, construction en cours avec
  tours restants et bouton « Annuler », liste des bâtiments constructibles (coût, durée, raison si indisponible)
  avec bouton « Construire », ressources de la province (icônes texte).
- Panneau faction (clic sur le blason de la barre) : trésor, revenu du dernier tour, revenu prévisionnel,
  entretien armées / bâtiments, sélecteur d'impôt (Bas / Normal / Haut) avec effet indiqué, biens disponibles
  par catégorie.
- Barre supérieure : « Revenu : +X (prév. Y) ».
- Carte : mode d'affichage (touche M) qui teinte les provinces par mécontentement moyen (vert → rouge) au lieu
  de la faction ; icône marteau sur les provinces en construction.
- Événements de révolte, peste, famine, bâtiment terminé dans le journal avec couleur dédiée.
- Smoke test : construire un marché à Paris, passer les tours nécessaires, vérifier qu'il apparaît dans
  `buildings` et que `projected_income` a augmenté ; passer l'impôt à Haut et vérifier la hausse de revenu.

## 4. Critères de fin
- 30 tours en France sans crash ; population de Paris évolue ; au moins 3 bâtiments construits ; une révolte
  déclenchable en imposant Haut dans une province occupée.
- Tests Rust verts (≥ 12 nouveaux), smoke Godot vert, captures `docs/img/godot-city-panel.png`.
