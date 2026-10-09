# WH — rapport `economie` (colonies, provinces, économie vs TWW3)

Lecture seule ; constats vérifiés dans `core/crates/sim-campaign/src/`, `data/`, `game/scripts/`. « RX » = déjà dans `docs/wip/rx/mecaniques.md` (trésors thésaurisés, arbre court, ordre public invisible, journal `Income` joueur seul) : cité, non reproposé.

## 1. Résumé

- Déjà proche de TWW3 : colonies prenables séparément avec chaînes de bâtiments (`building_slots.rs`, prérequis bâtiment/tech/ressource/côte/rivière/unicité dans `buildings.rs:786-840`), édits de province avec délai (`edicts.rs`), gouverneurs, impôt 3 paliers avec « fardeau » → mécontentement (`economy.json`), 4 classes sociales avec jauges, commerce par routes/hubs avec sécurité, sièges et embargos (`trade.rs`), aperçu avant/après des bâtiments (`preview.rs`, IB5), arbre de 45 techs en 3 branches, hérésie qui se propage (`religion.rs:511-600`).
- Manque le plus : (1) AUCUNE progression de colonie (pas de niveaux de ville/croissance : la taille d'une colonie est fixe, `SettlementKind` statique, tous les emplacements constructibles dès le départ, capacité de population plate 40 000) ; (2) l'ordre public et le revenu ne sont pas décomposés par cause pour le joueur (jauges à infobulle statique, revenu de colonie = un nombre) ; (3) impôt uniquement au niveau faction (pas par province) ; (4) pas d'équivalent de la corruption (influence de l'Église/hérésie à l'échelle province n'agit que par `heresy/5`) ; (5) édits gratuits et peu nombreux (5), aucune dépense d'influence/prestige ; (6) commerce quasi absent de l'UI de gestion.

## 2. Tableau des écarts

Impact 1-5 (joueur), coût S/M/L.

| # | Écart vs TWW3 | État actuel (fichier:ligne) | Imp. | Coût | Proposition | Dép. |
|---|---|---|---|---|---|---|
| E1 | Niveaux de colonie / croissance (village→bourg→ville, province tier 1-5) | `SettlementKind` fixe (`data-model/src/entities/settlement.rs:18-29`). Aucune règle de promotion ; `capacity` plate `BASE_CAPACITY = 40_000` (`buildings.rs:24`). Croissance de pop. existe (`population.rs:131-141`) mais ne débloque rien | 5 | L | « Rang » de colonie 1-4 par seuil de population de la province + bâtiment (voir §4-A) : débloque emplacements et bâtiments supérieurs, modifie `weight`/capacité | E2, E3 |
| E2 | Emplacements limités par niveau (arbitrage de construction) | Emplacements = nombre de chaînes autorisées : ville 20, bourg 19, château 10, abbaye 13, village 10 (calcul sur `data/buildings`, `building_slots.rs:60-110`). Aucun plafond : le joueur construit tout, le choix « spécialiser » n'existe pas | 5 | M | Plafond de bâtiments par colonie = f(rang) (ex. ville rang 1 : 4, rang 4 : 8 ; village 2-3) dans `data/rules/economy.json` `slots_by_rank`; refuser dans `build_blocker` (`buildings.rs:761`) avec raison « emplacements pleins » | E1 |
| E3 | Capitale de province vs colonie mineure | Cité = propriétaire de la province (`settlements.rs:42-55`); les autres colonies ont le même catalogue réduit par `settlement_kinds` et un `province_effect_percent` / `research_percent` par type (`buildings.rs`, `research.rs:285`). Pas de rôle « capitale » : pas d'édit/bâtiment spécifique, pas de bonus de capitale de faction hors `domain_income` 150 (`economy.rs:498-514`) | 3 | M | Chaînes « capitale de province » (hôtel de ville/prévôté, cour du bailli) réservées à la `city` (`settlement_kinds:["city"]` existe déjà, donc donnée seulement) ; bâtiments mineurs (grange, moulin banal, péage) pour village/château/abbaye | E2 |
| E4 | Ordre public lisible par composantes | `unrest_target` somme impôt, dévastation, biens, santé, garnison, occupation, religion étrangère, désordre, bâtiments (`population.rs:237-270`) ; `equilibrium()` donne la cible mais pas les termes (`population.rs:379`). UI : jauge + infobulle statique (`rich_tooltip.gd:836-853`), aucune liste « +24 occupation, +12 impôt, −8 garnison » | 5 | S/M | `unrest_breakdown(province) -> Vec<(label, f64)>` réutilisant les mêmes termes (refactor de `unrest_target` en retournant un struct de termes) ; exposé par le pont (`get_province_city`) ; liste dans l'infobulle de la jauge | — |
| E5 | Décomposition des revenus par source / par colonie | `FactionEconomy` agrégé (`economy.rs:69-113`), tableau 6 rubriques (`budget_table.gd:14-21`). Revenu par colonie = un entier (`settlement_panel.gd:296`). `province_income` (`economy.rs:178-215`) mélange impôt, production, commerce de bâtiments sans exposer les parts. Pas de ligne domaine (`domain_income` 150) ni routes commerciales par route côté UI | 4 | M | Struct `IncomeBreakdown { poll_tax_by_class, production, building_trade, building_tax, domain, routes, devastation_loss, tax_rate_mult }` par province et agrégée ; fenêtre « Finances » par colonne source ; infobulle du revenu de colonie | E4 (même style) |
| E6 | Impôt par province (et édits fiscaux) | `faction.tax_rate` unique : Bas/Normal/Haut pour toute la faction (`economy.rs:470-480`, `faction_panel.gd:9,131`). TWW3 : taux de province (impôt ×, ordre public, croissance) | 4 | M | `ProvinceState.tax_override: Option<TaxRate>` (défaut faction), ordre `SetProvinceTax`, `settlement_tax_with` (`economy.rs:310-348`) lit le taux local ; IA : relâcher les provinces à troubles élevés. Test : une province « Bas » baisse l'unrest de `tax_unrest_weight × Δburden` | — |
| E7 | Édits/commandements : nombre, coût, conditions | 5 édits, tous gratuits (`edicts.rs:7` « free »), un par province, délai 0-2 tours ; aucune condition de bâtiment/tech/foi, aucun coût en prestige/piété. Pas d'édit de croissance, de défense, de culture, de commerce armé, de recrutement | 3 | S (données) / M | +8 édits historiques : « Corvées de fortifications », « Foire franche », « Aumône royale », « Inquisition » (−hérésie, +unrest nobles/bourgeois), « Remise de la gabelle », « Bannalité » …; champs `cost` (livres/saison, prestige) et `requires` (bâtiment/tech) dans `data/edicts` + `Edict` data-model | E8 |
| E8 | Équivalent de la corruption | Seul vecteur : `heresy` (0-100) → `unrest += heresy/5` (`religion.rs:297`), bâtiments religieux la réduisent, propagation > 30, révolte > 70 (`religion.rs:18-23`). Excommunication : +10 unrest (`religion.rs:283`). Aucune « corruption » économique (prévarication des officiers, évasion fiscale, `tax_efficiency` fixe 0.082) | 3 | M | « Prévarication » par province : 0-100, croît avec distance à la capitale (hop) × nombre de provinces (lié à `administration_rate`), baisse avec bailli/gouverneur compétent (`governance`), cour de justice, enquêteurs (agent). Effet : réduit `tax_efficiency` local (`economy.rs:213`) et monte l'unrest. Fait remplacer l'administration plate 8-28 % (`economy_rules.rs:288`) par une pression locale lisible | E5 |
| E9 | Ressources et chaînes de production (marchandises) | 10 ressources (`data/resources`), `goods_satisfaction` par catégorie (luxe/denrée) ; production = % sur l'impôt (`production_tax_share` 0.5, `economy.rs:196-205`). Ressource requise par certains bâtiments (pierre, fer). Pas de stock ni échange de biens | 3 | M | Marchandises échangeables via routes (`trade.rs` valorise déjà `goods` des hubs) : bonus « bien rare » par province consommé par bâtiments d'artisanat (draperie→laine) au lieu du seul booléen `required_resource` | E5 |
| E10 | Commerce : gestion UI et leviers | Moteur riche (distance, sécurité, accord +30 %, embargo, mer, monnaie : `trade.rs:1-30`). UI : `faction_panel.gd`/`season_report.gd` seulement ; pas de carte/liste de routes avec valeur et menace, ni action « protéger/escorter une route », « établir un comptoir » | 3 | M | Onglet « Commerce » : liste de `trade_routes()` (valeur, hubs, sécurisée oui/non, cause de coupure) ; ordre « comptoir » dans un hub étranger (bâtiment `bld_trading_post` partageant la valeur) | E5 |
| E11 | Arbre technologique : forme et contenu | 45 techs, 3 branches, tiers 1-5 mais seulement 1 tech tier 5 par branche (compte `data/technologies`), coût 100-1 350 ; points de recherche = 5 + bâtiments + gouverneur (`research.rs:262-330`) ; coût +25 % si anachronique (`research.rs:99`). Recherche file 3 (`economy.json research_queue_max`). RX : saturée par petites factions | 3 | M | Déjà RX (tiers 4-5, coûts croissants). Ajouter ici : techs **spécifiques** (une tech « choix exclusif » par palier : ordonnances vs mercenaires) et techs à bonus d'économie de province (rotation triennale existe) ; vue arbre par branche avec prérequis (déjà `tech_tree_view.gd`) | RX |
| E12 | Coûts d'entretien lisibles et proportionnels | Entretien bâtiments par bâtiment (`upkeep` 5-60) ; administration = part du revenu (8 % + 0,7 %/province, plafond 28 %) ; garnison à 50 % (`economy.json`). Pas d'entretien ni d'effet de « hors capitale » ; l'administration croît sans choix du joueur | 3 | S | Voir E8 : remplacer ou doubler par prévarication locale ; exposer dans le tableau (ligne « Administration » déjà là : ajouter sous-ligne par province) | E8 |
| E13 | Pillage/dévastation → économie (ressenti) | Dévastation réduit revenu (`economy.rs:213`), croissance bloquée > seuil (`population.rs:40`), unrest/2. Pas de reconstruction active (« remise en état », convoi de grain) | 2 | S | Édit « Remise en état » : divise `devastation_decay` ×2 pour un coût en livres par saison | E7 |
| E14 | Événements/missions d'économie par province | Événements de chronique génériques (153) avec effets de trésor adaptés au revenu (`economy.json` description). Pas d'événement local « disette », « émeute du pain », « foire » couplé aux jauges d'une province | 2 | S | Événements gated par `goods_satisfaction < 30` ou `unrest > 60`, choix du joueur (distribuer du grain / réprimer) — données `data/events` | — |
| E15 | Construction : file et vitesse | File de 3 par colonie (`construction_queue_size`), durée en tours, vitesse via effets (`buildings.rs:646`). Trop permissif : coûts 300-6 000, l'argent n'est pas un frein (RX trésors) ; pas de « coût de la file » | 2 | S | Coûts indexés sur le rang (E1) ; suppléments « chantier seigneurial » : −X % pour la capitale | E1 |

## 3. Top 10 (impact / coût)

**1. E4 — Décomposition de l'ordre public (S/M, impact 5).**
Fichiers : `population.rs`, godot-bridge (`get_province_city`), `rich_tooltip.gd`, `classes_section.gd`.
- Extraire de `unrest_target` (`population.rs:237`) une fonction `unrest_terms(...) -> Vec<UnrestTerm { key, value }>` ; `unrest_target` = somme bornée de ces termes (aucun changement de règle).
- Champ `unrest_breakdown: Vec<(String,f64)>` pour la classe sélectionnée dans la réponse de `province_city`.
- Infobulle de jauge : liste signée des termes ≥ 0,5 + cible d'équilibre (`equilibrium()`).
- Tests : somme des termes bornée = `GaugeTargets.unrest` pour 3 provinces (occupée, taxée « Haut », paisible).

**2. E5 — Décomposition des revenus (M, impact 4).**
- `IncomeBreakdown` dans `economy.rs` rempli par `province_income` (même arithmétique, accumulateurs) ; somme == `province_gross_income` (test d'invariance).
- `FactionEconomy.income_by_province: Vec<(province, IncomeBreakdown)>` + `domain_income`, `routes` (déjà `trade_income`).
- UI : infobulle sur « Revenu » (`settlement_panel.gd:296`) et sous-lignes dépliables de `budget_table.gd`.
- Tests : somme des lignes = `projected_income` ; ligne domaine = 150 (`economy.json`).

**3. E6 — Taux d'impôt par province (M, impact 4).**
- `ProvinceState.tax_override: Option<TaxRate>` (sérialisé, `#[serde(default)]`), `Order::SetProvinceTax { province, rate }` validé comme `set_edict` (`edicts.rs:173`, province contrôlée, 1/tour).
- `settlement_tax_with` (`economy.rs:310`) et le fardeau (`province_inputs`, `population.rs:302`) lisent `tax_override.unwrap_or(faction.tax_rate)`.
- IA : `ai_choose_edicts`-like pass : « Bas » si unrest > 60, « Haut » si < 25 et pas en guerre.
- Tests : province Bas rapporte 0,7× et réduit la cible d'unrest ; migration de sauvegarde (`save.rs`).

**4. E7 — Édits plus nombreux et coûteux (S, impact 3).**
- Ajouter `cost: Option<{money,prestige}>` et `requires` (building/tech/religion) à `Edict` (data-model + schéma `data/schemas`), vérifier dans `set_edict` et prélever par saison dans `resolve_economy`.
- 8 nouveaux fichiers `data/edicts/*.json` avec sources historiques ; `ai_choose_edicts` tient compte du coût.
- Tests : édit refusé sans prérequis, coût prélevé, annulation si trésor < 0.

**5. E2 — Plafond d'emplacements par colonie (M, impact 5).**
- `economy.json` `slots_by_kind: {city:6, town:5, castle:3, abbey:4, village:2}` (valeurs initiales, avant E1) ; `build_blocker` (`buildings.rs:761`) : refuse un nouveau slot au-delà du plafond ; `building_slots.rs` expose `slots_used/slots_max`.
- Rebalance IA : `buildable_with_supply` ordonne par valeur (déjà PB3f). Vérifier la sonde `campaign_probe` (ADR 0246) : revenu et unrest dans les bandes.
- UI : « Emplacements 4/6 » dans `settlement_panel.gd` et `holdings_controller.gd:229`.

**6. E1 — Rang de colonie (L, voir §4-A).** Premier jet : rang dérivé (pas de nouvelle donnée) de la population de la province et des bâtiments, utilisé par E2.

**7. E3 — Bâtiments propres à la cité de province (S/M, impact 3).**
- 4 bâtiments `settlement_kinds:["city"]` (hôtel de ville, bailliage, halle aux grains, prévôté) + 4 pour colonies mineures ; effets existants (`tax_income`, `unrest`, `garrison`, `goods_satisfaction`), aucun code nouveau.
- Test de données : `data/schemas` + test `building_slots` (la ville a plus de racines que le village).

**8. E8 — Prévarication (M, impact 3, voir §4-B).**

**9. E10 — Onglet Commerce (M, impact 3).**
- Bridge `get_trade_routes` → `trade_routes()` + `RouteView` (`trade.rs:66`) : valeur, mode (terre/mer), `cut_reason`.
- `game/scripts/ui/trade_panel.gd` (liste triée par valeur, filtre « mes routes »), pas de logique de règle.
- Tests : GUT/`smoke_ui` + test Rust qui la vue = `faction_trade_income`.

**10. E14 + E13 — Événements locaux d'économie et remise en état (S, impact 2).**
- 6 événements `data/events` conditionnés par jauges ; édit « Remise en état » (voir E7).

## 4. Refontes lourdes

### A. Niveaux de colonie et croissance (E1, L)
Constat : `SettlementKind` est une propriété statique ; les chaînes de bâtiments sont les seuls progrès (`building_slots.rs`). TWW3 : une colonie évolue par niveaux (population/bâtiment) qui débloquent emplacements, bâtiments et rôle.
Proposition historique (sans magie) :
- `SettlementState.rank: u8` (1-4), calculé chaque saison depuis `population.total()`, la capacité (E2) et un bâtiment « charte » : village → bourg (charte de franchise) → ville → cité (évêché/université). Seuils en `data/rules/settlements.json`.
- Effets : emplacements (E2), `weight_share` (`settlements.rs:16`), `province_effect_percent`, stationnement de garnison, fortification max, recrutement (déjà `recruit_slots`).
- `capacity` par province dépend du rang des colonies (au lieu de la constante 40 000, `buildings.rs:24`).
- Événement chroniqué « X obtient sa charte de ville » ; modèle 3D : variante de maquette existante (familles de maquettes GC, ADR 0158).
- Risques : équilibrage (sonde ADR 0246), sauvegardes (`save.rs` migration), maquettes de rang. Jalons : (1) rang dérivé lecture seule + UI, (2) plafonds d'emplacements, (3) seuils de promotion, (4) bâtiments de rang.

### B. Prévarication / corruption historique (E8, M-L)
- Jauge par province 0-100 (`ProvinceState.misrule`) : + distance en sauts de graphe à la capitale (`reach.rs`/`path_plan.rs`), + nombre de provinces de la faction, + prélèvement « Haut », + régence (`regency`) ; − bailli à `governance` élevée, cour de justice (nouveau bâtiment), enquêteur (agent `agents/`), édit « Enquête des commissaires ».
- Effets : `tax_efficiency` local × (1 − misrule/200), unrest + misrule/10.
- Remplace/atténue `administration_base/per_province/max` : l'administration plate devient diffuse et lisible. Lien Église : évêque fidèle réduit la prévarication, hérésie l'augmente (couple `heresy`, `religion.rs`).
- Risques : complexité de lecture ; à lier à E4/E5 (breakdown) avant mise en jeu.

### C. Marchandises échangeables (E9 + E10, L)
Stock par faction des 10 ressources (`faction.goods`, déjà là), prix variables (`base_price`, `res_*`), vente aux hubs via les routes (`trade.rs`), consommation par bâtiments d'artisanat. Dépend du moteur commerce existant ; pas à engager avant E5/E10.

## 5. Points déjà bons (à ne pas toucher)
- Aperçu avant/après de construction et de technologie (`preview.rs:239-300`), dont `equilibrium()` : base idéale pour E4.
- Impôt à fardeau (`tax_unrest_weight`) et garnison par habitant (LR-07, ADR 0180) : cohérents et réglables en données.
- Moteur de routes commerciales (sécurité, mer, monnaie, embargo) : plus profond que TWW3 sur le plan historique ; manque seulement l'UI et un levier d'investissement.
- Hérésie propagée entre provinces voisines avec seuil de révolte : bonne base d'« influence de l'Église » ; il suffit d'ajouter un coût d'inquisition/évêque (E7, E8).
