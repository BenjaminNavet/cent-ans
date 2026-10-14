# CC C6 — échelle, normalisation, textes, équilibre (branche `cc/c6`)

## Plan
1. Échelle (ADR 0357) puis recalage points_per_step, max_muster_km, rivers.json, seuils km.
2. Renormalisation des champs additionnés à la fusion (règle écrite dans `cc-c3-donnees.md`).
3. Données : fac_navarre / title_check ; réécriture rejouable des textes (anciens noms).
4. Objectifs de victoire et scénarios (provinces disparues).
5. IA : filtre Attack vers cellule infranchissable ; arbitrage Abbeville/Meaux (c7a).
6. Lever les tests `CC C6: balance` (7 : jr4b, campaign.rs:714, cm2, om3, a6_l2, a6_forecast, rg10).
7. Seed bots rapide puis longue ; constats dans le rapport.

## État (wip 1)
Fait :
- ADR 0357 : `meters_per_px` nominal conservé ; points_per_step 250 -> 155, max_muster_km 300 -> 350, km_per_turn
  150 -> 260, rayons de repli 2/1 -> 3/2 pas, km des fleuves (script `cent-ans compose c6 rivers`).
- base_supply renormalisé (`compose c6 base-supply`, règle dans cc-c3-donnees.md § C6).
- Textes : table `TEXT_FIXES` (missions, sacre de Reims, tutoriel) ; bg_picardie -> prov_ponthieu.
  Audit (`compose c6 audit`) : ~1600 mentions d'anciens noms restent, 97 % en prose historique (maisons
  Valois/Luxembourg/Anjou/Foix, personnes, récits, codex) : volontairement non réécrites.
- fac_navarre : titres Angoulême/Mortain retirés.
- IA : `Attack` filtré par `find_path` (grid.rs).
- Tests recalés : c7a_retreat / review_tests (neutral 3 pas), campaign movement_spans (portée dérivée), starting_fit.

À faire : lever les tests `CC C6: balance` (jr4b, campaign.rs:714, cm2, om3, a6_l2, a6_forecast, rg10),
seed bots, pytest/ruff, smoke.gd headless.
