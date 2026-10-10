# WH econ — état (FAIT, à fusionner)

Branche `wh/econ` (worktree `../gp-wh-econ`). Spec : `docs/wip/wh/economie.md` § 3 points 1, 2, 3, 4, 5, 7. ADR 0274, 0275.

- 1 ordre public décomposé : `population::unrest_terms` / `unrest_breakdown`, pont `classes.*.unrest_terms`, infobulle de jauge.
- 2 revenus par source : `income_breakdown.rs` (somme invariante testée), ponts `get_income_breakdown`, `income_lines`
  (`get_faction_economy`, `settlement_detail`), sous-lignes du budget (`budget_table.gd`), infobulle du revenu de colonie.
- 3 impôt par province : `province_tax.rs`, `Order::SetProvinceTax`, `ProvinceTaxSection` (panneau de province), passe IA
  `plan_province_taxes` (seuil `ai/campaign.json` `province_tax_relief_unrest`, 70 depuis WR ai-diplo, ADR 0302 : l'IA lit `AiCampaign::bundled()`, compilé, et l'essai à 70 n'avait pas recompilé).
- 4 édits : `cost {money, prestige}`, `requires {building, technology, religion}`, 8 nouveaux édits (`data/edicts/`), prix
  prélevé dans « Cour et administration », abandon des édits trop chers si le trésor ne suit pas.
- 7 bâtiments de cité (hôtel de ville, cour du bailli, halle aux grains, prévôté) et de colonies mineures (grange dîmière,
  four banal, péage, chapelle seigneuriale).
- 5 plafond d'emplacements `building_slot_cap` (`data/settlements/rules.json`) : cité 10, ville 5, château 4, abbaye 4, village 3.

## Mesures campaign_probe (120 tours)
Avant (graines 1,2, binaire d'origine) : révoltes 1 et 8, guerre FR-EN 66 et 92 %, banqueroutes 122 et 87, revenu France
32 912 / 26 071. Proxy « avant » (mêmes données sans plafond/bâtiments/édits) graines 3-6 : révoltes 1,0,3,0.
Après (graines 1-6) : révoltes 0,1,0,2,1,2 ; guerre FR-EN 59-82 % ; banqueroutes 66-119 ; revenu France 22-42 k.
Variantes : plafond serré 7/4/3/3/2 : révoltes 1 et 3, mêmes revenus ; sans plafond : 0 et 2. Les 8 bâtiments adoucis
(apaisement réduit : hôtel de ville -2, prévôté -3, bailli 0, chapelle -1) après un premier essai à 0 révolte.
Les révoltes étaient déjà sous la bande 4-10 avant le lot (point ouvert connu), le lot n'y change rien de mesurable.

## Reste / points ouverts
- Pas de rang de colonie (E1), de prévarication (E8) ni d'onglet commerce (E10) : hors lot.
- Aucune capture : seul le test headless `game/tests/wh_econ_test.gd` (OK) ; l'allure de la section d'impôt et des
  sous-lignes du budget n'est pas contrôlée à l'œil.
- Cap de cité à 10 volontairement large (voir ADR 0275) : à resserrer si le joueur veut un vrai arbitrage.
