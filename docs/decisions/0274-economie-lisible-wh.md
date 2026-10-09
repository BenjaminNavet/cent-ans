# 0274 — Économie lisible : décompositions, impôt par province, édits à coût

Statut : accepté

## Contexte
Lot WH `econ` (rapport `docs/wip/wh/economie.md`, § 3 points 1, 2, 3, 4). Le joueur voyait des jauges et des revenus sans
leurs causes : le mécontentement n'avait qu'une infobulle statique, le revenu d'une colonie était un entier, l'impôt ne
se réglait que pour toute la faction et les cinq édits régionaux étaient gratuits et sans condition.

## Décision
- **Ordre public décomposé.** `population::unrest_terms` renvoie les termes signés de la cible de mécontentement d'une
  classe (impôt, dévastation, biens, santé, garnison, occupation, religion étrangère, désordre, bâtiments, loyauté) ;
  `unrest_target` en est la somme bornée, sans changement de règle. `unrest_breakdown(province)` les expose, le pont les
  ajoute à `get_province_city().classes.*.unrest_terms` et l'infobulle de la jauge les liste (termes d'au moins 0,5).
- **Revenus par source.** `income_breakdown.rs` décompose `province_income` en dix sources (taille par classe,
  production, taux d'imposition, recettes et commerce des bâtiments, dévastation, province entière) en flottants, arrondit
  chaque ligne et met le reliquat d'arrondi sur la plus grosse : la somme des lignes égale toujours
  `settlement_tax().round()`, `province_gross_income` et `faction_income` (testé). Au niveau de la faction s'ajoutent
  embargos, domaine, difficulté et aumônes de croisade ; le budget y ajoute seigneuriage et routes commerciales.
- **Impôt par province.** `ProvinceState.tax_override: Option<TaxChoice>` (taux, faction, tour ; suit les règles des
  politiques de province : caduc si le contrôleur change, un changement par tour) et `Order::SetProvinceTax`. Tout
  lecteur du taux passe par `province_tax_rate` (impôt et fardeau du mécontentement). L'IA allège à « Bas » une province
  agitée (au-delà de `low_tax_min_unrest`) quand le taux du royaume est plus lourd, et la relâche une fois calme ; elle
  ne relève jamais une seule province (le taux général reste à `plan_tax_rate`).
- **Édits à coût et prérequis.** `Edict.cost { money, prestige }` : `money` en livres par saison tant que l'édit est en
  vigueur, versé dans la ligne « Cour et administration » du budget (aucune ligne de budget nouvelle) ; `prestige`
  dépensé une fois par le souverain à l'adoption. `Edict.requires { building, technology, religion }` : bâtiment dans la
  cité de la province (ou amélioration), technologie ou foi du contrôleur ; un édit dont les prérequis disparaissent
  cesse. Un trésor qui ne peut porter le budget abandonne d'abord les édits les plus chers. Huit édits historiques
  ajoutés avec sources (corvées de fortifications, foire franche, aumône royale, Inquisition, remise de la gabelle,
  banalités, hôtes et défrichements, enquêteurs-réformateurs). L'IA pondère le prix et n'adopte un édit payant que s'il
  coûte au plus 4 % du dernier revenu et si le trésor le couvre trois saisons.

## Conséquences
- Aucun effet d'équilibre pour les points 1 et 2. Les points 3 et 4 n'en ont que par le jeu des édits et de l'IA
  (mesuré avec `campaign_probe`, voir ADR 0275).
- Nouveaux champs sérialisés avec `#[serde(default)]` (anciennes sauvegardes lisibles).
- Reste hors lot : la prévarication (corruption), l'onglet commerce, le rang de colonie.
