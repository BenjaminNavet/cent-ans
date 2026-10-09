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
- Mesures `campaign_probe` avant/après : voir `docs/wip/wh/econ.md`.
- Un plafond trop bas fait chuter revenus et apaisement ; il se règle dans les données seules.
