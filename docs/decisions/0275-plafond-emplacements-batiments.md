# 0275 — Plafond d'emplacements de bâtiments et bâtiments de cité

Statut : accepté

## Contexte
Lot WH `econ`, points 5 et 7. Toute colonie pouvait construire toutes ses chaînes de bâtiments (20 dans une cité, 10
dans un village) : le joueur bâtissait tout, sans arbitrage. La cité de province et les colonies mineures partageaient
le même catalogue.

## Décision
- `data/settlements/rules.json` `building_slot_cap` : nombre maximal de chaînes de bâtiments tenues à la fois, par type
  de colonie. Une chaîne en cours (chantier ou file) compte ; améliorer un bâtiment réutilise son emplacement ; ce qui
  dépasse déjà le plafond est conservé (seules les nouvelles chaînes sont refusées, raison « emplacements pleins
  (n/m) »). Les valeurs sont reprises du résultat des mesures ci-dessous.
- `CampaignState::slot_usage` donne `{used, max}`, exposé par `settlement_slot_usage` ; la bande des emplacements
  affiche « Emplacements n/m ».
- Huit bâtiments : hôtel de ville, cour du bailli, halle aux grains, prévôté (cité seulement) et grange dîmière, four
  banal, péage, chapelle seigneuriale (colonies mineures, jamais la cité). Effets existants, aucun code nouveau.

## Conséquences
- Valeurs retenues : cité 10, ville 5, château 4, abbaye 4, village 3. Au départ une cité compte au plus 11 bâtiments
  (médiane 4-5) : un plafond de cité à 8 bloquait d'emblée 33 cités (Devon pleine à 8/8) et cassait deux tests, d'où
  10 ; des plafonds plus bas (7/4/3/3/2) ont donné les mêmes bandes.
- Mesures `campaign_probe` (120 tours, 6 graines, avant = mêmes données sans plafond, bâtiments ni édits ajoutés) :
  révoltes 2,2/partie avant (déjà sous la bande 4-10, point ouvert connu) contre 1,0 après (bruit de graine : 0 à 8
  d'une partie à l'autre) ; guerre FR-EN 63-92 % avant, 57-82 % après ; revenu de la France à 120 tours 26-42 k avant,
  22-42 k après ; banqueroutes 66-137 contre 70-126. Rien qui sorte des bandes du fait du lot ; détail dans
  `docs/wip/wh/econ.md`.
- Un plafond trop bas fait chuter revenus et apaisement ; il se règle dans les données seules.
