# 0346 — Fenêtre Économie avec onglet Commerce

## Contexte
Retour du joueur (RG, 11/10) : l'économie n'a pas de fenêtre propre (elle passe par le panneau de
faction), `get_income_breakdown` est exposé mais jamais appelé, le bouton « Commerce » de la
diplomatie ne fait qu'afficher le calque des routes, et les accords commerciaux vivent dans le
traité de la diplomatie. RG3 a ajouté la projection de gain d'un accord
(`get_trade_agreement_projection`) et le symbole `₶` ; RG8 les prévisions du prochain tour
(`get_faction_forecast`) ; le cœur garde déjà l'historique du budget (`budget_history`, 12 saisons).

## Décision
1. **Fenêtre Économie** (`EconomyWindow`, panneau central enregistré par `EconomyController` via
   `MapUI.register_panel`, donc redimensionnable par la poignée de RG1). Bouton « Économie » (₶)
   dans la barre du haut après « Techniques » ; touche **M** (`map_toggle_economy`, libre dans
   `project.godot` ; ajoutée à la fiche des raccourcis). Trois onglets :
   - **Vue d'ensemble** : trésor et trésor du prochain tour, tableau recettes / dépenses / solde
     (`BudgetTable`, recettes ventilées par `get_income_breakdown`), prévisions de prestige, hommes
     et troubles avec leurs principales causes (`get_faction_forecast`, `get_income_top_provinces`),
     historique du solde des 8 dernières saisons (`budget_history` du cœur).
   - **Commerce** : accords en cours avec leur gain par saison et « Rompre » (confirmation en deux
     clics) ; « Nouvelles routes » = partenaires qui partagent une route avec nous, triés par gain
     projeté, avec la projection pour les deux camps avant signature et « Proposer l'accord » ;
     routes actives du joueur (même texte que l'ancienne fenêtre des routes) ; biens produits ;
     « Voir sur la carte » qui active le calque des routes.
   - **Monnaie** : livre tournois (`₶`), niveau de frappe, indice des prix, tableau des quatre
     niveaux de frappe (seigneuriage, refonte, inflation, troubles bourgeois, prestige) d'après
     `get_coinage`, et l'explication des sources de revenu. Lecture seule : on change de frappe
     dans la section « Monnaie » du panneau de faction.
2. **Un getter de cœur**, la seule donnée qui manquait : `trade::trade_partner_options` (pont :
   `get_trade_partner_options(faction)`) liste les partenaires de routes avec
   `{gain_self, gain_partner, current_self, signed}`, triés par gain décroissant ; sans cela l'UI
   aurait dû appeler la projection pour ~130 factions. Aucun nouvel ordre : « Proposer » et
   « Rompre » émettent les ordres existants `propose_treaty` (article `trade_agreement`) et
   `break_trade_agreement`, validés par le cœur comme avant.
3. **Liens** : le bouton « Commerce » de la diplomatie devient « Commerce → Économie » et ouvre
   l'onglet Commerce ; la touche `I` (`campaign_commerce`), l'avis « commerce » et le bouton du
   panneau de faction (« Économie et commerce ») ouvrent le même onglet. L'ancienne fenêtre des
   routes n'est plus ouverte nulle part ; `TradePanel.route_text` reste la mise en texte d'une route
   (réutilisée). Le bouton de la barre sous la mini-carte et la touche `V` restent le simple
   interrupteur du calque. La proposition d'accord *dans* un traité de la diplomatie (article
   « Accord commercial » avec sa projection, RG3) est conservée : un traité peut grouper
   plusieurs clauses.
4. **Icônes** : un seul point de résolution, `EconomyIcons` (notion → identifiant `IconLibrary`,
   biens → `res_*`). RG14 ajoute des icônes de ressources dans `icons.json` : elles remplacent les
   actuelles sans toucher à la fenêtre.

## Hors champ (et pourquoi)
- **Échanges de ressources** (acheter / vendre une ressource contre de l'or au prix du marché) : le
  cœur ne tient aucun stock de ressources (`goods` d'une faction compte les lieux de production, pas
  des quantités ; les routes valent un revenu, pas des marchandises). Créer un marché, des prix et
  des stocks serait un système entier, sans effet sur le reste du jeu. Pas d'ordre
  `ExchangeResources` ; la fenêtre le dit en une ligne. À reprendre si un jour le ravitaillement ou
  la production gagnent des stocks.
- **Règles de change** : le cœur n'a pas de change entre monnaies, seulement la frappe
  (seigneuriage, refonte, inflation) ; l'onglet Monnaie l'affiche en lecture seule.
- **Prévision du solde par cause** au-delà des cinq premières causes (`ForecastText.TOP`).

## Conséquences
- Les règles restent dans `core/` ; `game/` affiche et émet des ordres.
- La touche M est la touche physique ; sur AZERTY la fiche des raccourcis l'affiche comme « , »
  (même correspondance que le reste de la fiche).
- Tests : `core/crates/sim-campaign/tests/economy/c5_trade.rs` (options de partenaires),
  `game/tests/rg12_economy_panel_test.gd`.
