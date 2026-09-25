# SV4 — chiffres de règles écrits en dur dans l'UI

Branche `sv4-ui-numbers` (worktree, non fusionnée). Origine : `docs/wip/suites-bulles3.md`.

## État
- [x] Audit des GDScript (liste ci-dessous)
- [x] Constantes de ravitaillement de `economy.rs` → `data/rules/economy.json` (+ schéma, défauts `EconomyRules`)
- [x] Accesseur générique `GameDataStore.get_rule_constants()` + helper GDScript `RuleValues`
- [x] Textes d'UI branchés sur les constantes
- [x] Vérifs : cargo fmt/clippy/test verts, `core/build.sh`, import Godot, smoke vert, pytest 561 verts

## Mécanisme
- Cœur : `sim_campaign::rule_constants::rule_constants(&GameData)` rend `{nom: valeur}` (pourcentages en %),
  chaque valeur lue dans une constante du cœur ou dans `data/rules` (jamais recopiée).
  Pont : `GameDataStore.get_rule_constants()` (`core/crates/godot-bridge/src/data_store_rules.rs`) y ajoute
  deux seuils de bataille (`exhausted_fatigue`, `breach_open_threshold`).
- UI : `game/scripts/ui/rule_values.gd` (`RuleValues`). Un texte écrit `{rule.nom}` ; `RuleValues.format()`
  remplace par la valeur au format français ; `{rule.nom:1}` impose une décimale (« ×1,0 »).
  `RuleValues.text(nom)` / `RuleValues.value(nom, repli)` pour les chaînes `%`. Sans données : « ? ».
- Constantes nommées au passage (sans changement de valeur) : `population::HEALTH_NEUTRAL`,
  `GOODS_TARGET_BASE`, `GOODS_TARGET_PER_CATEGORY`, `religion::DONATION_LIVRES_PER_FAVOR`,
  `movement::LANDING_WINTER_FACTOR`, `skills::MAX_SKILL_LEVEL`, `sim_battle::sim::EXHAUSTED_FATIGUE`,
  `sim_battle::siege::BREACH_ONE_GAP` / `BREACH_TWO_GAPS`.
- `data/rules/economy.json` : `supply_loss` 20, `supply_loss_winter` 35, `supply_recovery` 40,
  `starvation_loss_percent` 10, `devastation_decay` 5 (ex-`ATTRITION_SUPPLY_LOSS*`, `SUPPLY_RECOVERY`,
  `STARVATION_LOSS_PERCENT`, `DEVASTATION_DECAY`).

## Audit — chiffres corrigés (lus dans le cœur désormais)
Numéros de ligne d'avant correction.

| Fichier:ligne | Chiffre en dur | Source de vérité |
|---|---|---|
| ui/rich_tooltip.gd:50-51 | Carême −3 piété, +10 mécontentement | `table::LENT_PIETY_PENALTY`, `LENT_CLERGY_UNREST` |
| ui/rich_tooltip.gd:52 | Carême +2 piété | `table::LENT_FISH_PIETY` |
| ui/rich_tooltip.gd:54 | frais ×1,5 l'hiver | `table::WINTER_FRESH_COST_PERCENT` |
| ui/rich_tooltip.gd:88 | révolte > 75, 3 saisons, rebelles > 90 | `population.json` `revolt_*` |
| ui/rich_tooltip.gd:89 | santé 50 (cible, déclin), peste < 30 | `population::HEALTH_NEUTRAL`, `PLAGUE_HEALTH_THRESHOLD` |
| ui/rich_tooltip.gd:91 | biens 40 + 10 par catégorie | `population::GOODS_TARGET_BASE`, `GOODS_TARGET_PER_CATEGORY` |
| ui/rich_tooltip.gd:92 | croissance nulle > 50 de dévastation | `population::GROWTH_DEVASTATION_CAP` |
| ui/rich_tooltip.gd:100 | dette : −10 de moral | `economy.json` `bankruptcy_morale_penalty` |
| ui/rich_tooltip.gd:441 | import prix × 100 | `economy.json` `resource_import_multiplier` |
| ui/rich_tooltip.gd:486 | technologie + 25 % | `research::ANACHRONISM_SURCHARGE_PERCENT` |
| ui/tech_tree_view.gd:192 | + 25 % | idem |
| ui/ransom_panel.gd:234 | échéances total + 10 % | `ransom::INSTALLMENT_SURCHARGE_PERCENT` |
| ui/ransom_panel.gd:290 | impayé + 10 % | `ransom::DEFAULT_SURCHARGE_PERCENT` |
| ui/alerts.gd:132 | dette + 10 % | idem |
| ui/diplomacy_panel.gd:166 | faveur +1 par 200 livres | `religion::DONATION_LIVRES_PER_FAVOR` |
| ui/diplomacy_panel.gd:1038 | médiation 1 000 ₶ | `diplomacy::MEDIATION_COST` |
| ui/table_section.gd:16 | Carême 3 / +10 / +2 | `table::LENT_*` |
| ui/faction_panel.gd:134-136 | impôt ×0,7 / ×1,0 / ×1,4 | `economy::TaxRate::multiplier` |
| ui/encyclopedia.gd:58 | administration 8 % + 1 %/province, 35 % max ; 20 % au-delà de 6 saisons ; dette −10 | `economy.json` `administration_*`, `opulence_*`, `bankruptcy_morale_penalty` |
| ui/encyclopedia.gd:60 | pays dévasté « moitié plus / moitié moins » (+ pertes 20/35, reprise 40, famine 10 % ajoutées) | `economy.json` `supply_*`, `starvation_loss_percent` |
| ui/encyclopedia.gd:62 | 210 km, 140 l'hiver ; zone de contrôle 8 km | `settlements/rules.json` `movement` × `Season::movement_steps` ; `movement/rules.json` `zoc_radius_km` |
| ui/encyclopedia.gd:66 | compagnons « huit au plus » | `retinue.json` `max_per_character` |
| ui/encyclopedia.gd:67 | « plus de vingt ans » (+ 25 % ajouté) | `research::ANACHRONISM_YEARS` |
| ui/encyclopedia.gd:68 | « deux saisons pour choisir » | `chronicle::DECISION_TURNS` |
| ui/encyclopedia.gd:1113 | sceau 1 à 5, +2 réussite, +1 échec | `agents.json` `experience_thresholds`, `xp_success`, `xp_failure` |
| map/help_controller.gd:11 | débarquement 5 % (10 % l'hiver) | `movement::LANDING_LOSS_PERCENT`, `LANDING_WINTER_FACTOR` |
| map/help_controller.gd:12 | 20 %, six saisons, −10 de moral | `economy.json` |
| map/help_controller.gd:14 | « deux tours pour choisir » | `chronicle::DECISION_TURNS` |
| map/map_mode_controller.gd:51 | `SUPPLY_LOSS_MAX := 35.0` (filtre ravitaillement) | `economy.json` `supply_loss_winter` |
| battle/battle_unit_markers.gd:33 | `EXHAUSTED_FATIGUE := 60.0` | `sim_battle::sim::EXHAUSTED_FATIGUE` |
| battle/unit_card.gd:234 | idem (via la constante ci-dessus) | idem |
| battle/battle_hud.gd:346 | « Commandement %d / 10 » | `skills::MAX_SKILL_LEVEL` |
| battle/pre_battle_dialog.gd:434 | « Commandement %d / 10 » | idem |
| battle/pre_battle_dialog.gd:242 | brèche « ouverte » ≥ 50 | `sim_battle::siege::BREACH_ONE_GAP` |

34 emplacements, environ 60 chiffres. **Aucun chiffre n'était faux** par rapport au cœur. Seul écart :
le badge « épuisée » s'allumait à fatigue ≥ 60 alors que la simulation retire du moral au-delà de 60
(> 60) ; aligné sur le cœur. Le commentaire « vitesse réduite » était inexact (la vitesse baisse
continûment avec la fatigue ; 60 est le seuil de perte de moral).

## Audit — laissés (notés seulement)
- Hors lot (autres agents) : `map/province_panel.gd` (surcoût d'import, SV3) — aucun chiffre de règle
  en dur trouvé ; `visual/render_quality.gd` (SV3) — réglages de rendu ; recrutement/coûts d'unités (SV2)
  — les coûts viennent déjà du catalogue ; vision (SV1) — rien en dur dans l'UI.
- `ui/encyclopedia.gd:1102` : agents « un de moins l'hiver » et repli `movement_steps` = 3 — le « −1 »
  est codé en ligne dans `agents::agent_movement_allowance` (pas de constante) ; à nommer un jour.
- `ui/tutorial_steps.gd:157` : Bourgogne « l'échéance est 1477, l'année de Nancy » — date historique qui
  coïncide avec `victory.end_year` de `fac_burgundy` ; les conseils ne passent pas par `.format()`.
- `ui/character_sheet.gd:368` « Piété : %d / 100 », `rich_tooltip.gd:623` « %d / 100 » : échelle des
  jauges 0-100 (bornes `clamp(0, 100)` en ligne dans le cœur), laissé.
- `ui/diplomacy_panel.gd:38-43` : `GIFT_AMOUNT`, `DONATION_AMOUNT`, `TRIBUTE_SEASONS`, `TRUCE_TURNS`
  (« Trêve de deux ans ») : paramètres proposés par l'UI et transmis à l'ordre, pas des copies d'une règle.
- `ui/coinage_section.gd:140` « Prix de 1337 = 100 » : base d'indice, pas une règle.
- `ui/settings_menu.gd:205` libellés « × 0,5 … × 4 » : doublent `settings.gd` `UNIT_SIZES` (réglage de
  rendu, pas une règle).
- `ui/army_strip.gd:37` `DEFAULT_BASE_MORALE` : repli d'affichage si le catalogue manque.
- Visuels légitimes : `map/life_effects.gd` (seuils de ruines/fumées), `map/settlement_growth.gd`,
  `battle/battle_music.gd` (moral critique pour la musique).
- Cœur : d'autres constantes économiques restent dans `economy.rs` (`TAX_EFFICIENCY`,
  `UPKEEP_MONTHS_PER_SEASON`, `GARRISON_*`, barème `tax_per_head`, `TaxRate`) : hors du périmètre
  « ravitaillement » demandé.

## Prochaine étape
Fusion par l'orchestrateur (`integration/suites3`). Conflits possibles : `rich_tooltip.gd` (ligne import
× 100, B7c/SV3), `economy.json` / schéma (si SV2/SV3 y ajoutent des clés).
