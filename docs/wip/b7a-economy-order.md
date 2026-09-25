# B7a — incohérences code ↔ interface : économie et ordre

Branche `b7a-economy-order` (depuis main 04657c09). Liste d'origine : `docs/wip/bulles-partout.md`, « Incohérences code ↔ interface relevées par B2 ».

## Décisions
1. **Cour / opulence** : on garde le code (20 % de l'excédent au-delà de 6 saisons de revenu, réglé en F4 et vérifié par `m10_balance`). Les constantes d'administration, d'opulence et de banqueroute passent dans `data/rules/economy.json` (schéma `economy_rules.schema.json`). Aide F1 corrigée.
2. **Dette** : pas de débandade (le design m2 § 5 dit « trésor négatif → moral −10 ») : textes alignés sur le code.
3. **Ravitaillement « dévasté »** : le design général (§ ravitaillement : « attrition en hiver et en territoire ravagé ») le prévoit et l'état `ProvinceState.devastation` existe : implémenté dans `supply_modifiers` (hors pays ami, perte +50 % à dévastation 100 ; en pays ami, reprise −50 % à dévastation 100), paramètres dans `economy.json`. Placé dans `supply_modifiers` pour ne pas entrer en conflit avec `feat/map-modes` (qui refactorise `resolve_attrition`).
4. **Deux mécontentements** : déjà corrigé par EQ1 (79e0c7c4) — le pont expose `unrest` = moyenne pondérée des classes, `disorder` = jauge de troubles. Reste : le mock GDScript et les textes (fiche ordre public obsolète).
5. **Carte du mécontentement** : ratio divisé par 100 dans `campaign_map.gd` (la version MF1 de `feat/map-modes` le fait déjà).

## État
- [ ] squelette
- [ ] 1 constantes → données + textes
- [ ] 2 textes dette
- [ ] 3 ravitaillement × dévastation + tests + textes
- [ ] 4 mock + fiche ordre public
- [ ] 5 carte
- [ ] validations (cargo, build, smoke, pytest, codex)

## Prochaine étape
Implémenter le point 1 (EconomyRules).
