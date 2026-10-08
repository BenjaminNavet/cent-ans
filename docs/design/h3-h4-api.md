# H3 « La Table » et H4 « Médecine » — règles, données et API du pont

Conception : `docs/design/2026-09-23-histoire-et-savoir.md` § 2-3. Suivi : `docs/archive/chantiers.md`.
Règles : `core/crates/sim-campaign/src/table.rs` et `medicine.rs`. Pont :
`core/crates/godot-bridge/src/campaign_sim_table.rs` (nouveau) et `campaign_sim_tech.rs`.

## 1. Données

| Fichier | Contenu |
|---|---|
| `data/schemas/diet.schema.json` | Schéma d'un régime (`diet_…`, déclaré dans `common.schema.json`) |
| `data/diets/diet_*.json` | 7 régimes (§ 3) |
| `data/technologies/tech_*.json` | 12 techs de la branche `medicine` + 2 migrées (§ 4) |
| `data/buildings/bld_herb_garden.json`, `bld_apothecary.json` | Bâtiments sanitaires (§ 4) |
| `data/codex/_diet_links.md`, `_herb_links.md` | Fiches codex attendues par le lot codex |

Nouveautés de schéma : `technology.branch` accepte `medicine` ; champ optionnel `herbs: ["cdx_…"]` ;
effets `plague_resistance`, `wound_recovery`, `diet_health`.

Un régime : `name`, `description` (liens `[[cdx_…]]`), `requirements` (`resources` accessibles à la
faction — même notion que la satisfaction en biens : produites par une province contrôlée ou
alliée —, `coastal`, `technology`, `any_building` (au moins un), `terrains` (un parmi)),
`cost_per_thousand` (livres par saison pour 1 000 habitants), `effects` (effets existants, `class`
possible), `lent_rule` (`none|meat|fish|dairy`), `winter_rule` (`none|fresh`), `sources`.

## 2. Règles

**Régimes.** Chaque province mange `diet_bread_pottage` (neutre, gratuit) sauf choix de son
contrôleur (`set_diet`). Un choix fait par un ancien contrôleur ne lie pas le nouveau (retour au
défaut). Un changement par province et par tour ; l'effet s'applique à la résolution du tour.

- Coût = population / 1000 × `cost_per_thousand`, ×1,5 en hiver pour `fresh`. Payé dans la phase
  économique, ligne « Table » (incluse dans l'entretien total). Province par province (ordre des
  ids), un régime que le trésor (après revenus et autres entretiens) ne couvre pas revient au
  défaut, avec un message `table` au journal du joueur.
- Conditions revérifiées chaque tour après le calcul des biens : si elles ne tiennent plus
  (ressource perdue, technologie retirée, bâtiment disparu), retour au défaut + message.
- Effets : `health`, `growth`, `wealth`, `unrest`, `goods_satisfaction` (par classe si `class`)
  s'ajoutent à ceux des bâtiments/techs dans la population ; `health` est majoré par
  `diet_health` (%) des techs du contrôleur (*Regimen sanitatis* : +25 %) ; `army_morale` monte le
  moral des recrues levées dans la province ; `piety`/`prestige` vont au souverain une fois par
  saison (la meilleure valeur parmi les provinces, pour qu'un grand royaume ne les multiplie pas).
- Carême (printemps) : une faction ayant au moins un régime `meat`/`dairy` perd 3 de piété du
  souverain et le clergé de ces provinces gagne +10 de mécontentement ; un régime `fish` quelque
  part donne +2 de piété. Message `table` « Carême : … ».
- IA (`table::ai_choose_diets`, partagée par les deux planificateurs, ADR 0003) : en dette, tout
  revient au défaut ; sinon, dans chaque province au régime par défaut, le régime disponible au
  meilleur rapport effets/coût tant que le budget Table reste sous 10 % du revenu et sous 1/8 du
  trésor. Déterministe (ordre des ids).

**Médecine.**
- Recherche : **pas de recherche par branche**. Les points de recherche forment une réserve unique
  par faction (`ResearchPoints`, une seule tech à la fois) ; les effets `research_civil`/
  `research_military` existants ne sont lus par aucune règle. Un effet `ResearchMedicine` serait
  donc inutile (YAGNI) : la « recherche médicale » du document de conception est rendue par des
  `research_points` (théorie des humeurs +1, Montpellier +2, jardin des simples +0,5).
- IA de recherche : la branche qui compte le moins de techs acquises est préférée (égalité :
  militaire, civile, médecine), donc les trois arbres alternent.
- `plague_resistance` (points de %, bâtiments de la province + techs du contrôleur, plafond 50) :
  peste locale (santé moyenne < 30) — épargnée avec une probabilité r, sinon pertes 10 % × (1 − r)
  et mécontentement 20 × (1 − r) ; Peste noire — épargnée avec la probabilité r/2, pertes, santé et
  mécontentement × (1 − r) ; épidémie locale (`evt_epidemie_locale`, liste
  `medicine::EPIDEMIC_EVENTS`) — contenue avec la probabilité r, sinon effets `health` et
  `population` négatifs × (1 − r). Sans résistance, aucun tirage supplémentaire : le flux
  aléatoire des parties sans médecine est inchangé.
- `wound_recovery` (%, techs de la faction, plafond 50) : après une bataille de campagne
  (auto-résolue **ou** 3D, via `movement::apply_outcome`), chaque unité qui survit (non détruite)
  récupère ⌊pertes × r⌋ hommes, vainqueur comme vaincu. Message `medicine` pour le joueur.
- Migration : `tech_hospital_reform` et `tech_quarantine` passent dans la branche `medicine`
  (simple changement de `branch` ; prérequis croisés conservés avec `tech_urban_sanitation`,
  civile). `tech_quarantine` gagne `plague_resistance` +10. L'arbre civil garde 14 techs.

Nouveaux genres d'événements (`EventKind`) : `table` et `medicine` — à ajouter aux rubriques du
rapport de saison (`season_report.gd`, groupe « Royaume ») par la vague UI.

Sauvegardes : `ProvinceState.diet` et `FactionState.table_upkeep_last_turn` sont `serde(default)` ;
`STATE_VERSION` reste 4 et les anciennes sauvegardes se chargent.

## 3. Régimes

| Id | Nom | Conditions | Coût / 1000 hab. / saison | Effets | Carême | Hiver |
|---|---|---|---|---|---|---|
| `diet_bread_pottage` | Pain bis et potage | — | 0 | — | — | — |
| `diet_pulses` | Fèves, pois et lentilles | `tech_three_field_rotation` | 0,08 | santé paysans +3, croissance +5 % | — | — |
| `diet_lenten_fish` | Hareng saur et poisson de carême | `res_fish` + `res_salt` | 0,15 | santé +2, biens clergé +4 | fish (+2 piété) | — |
| `diet_meat_salting` | Lard, bœuf et salaisons | `res_salt` | 0,35 | santé +5, mécontentement paysans −4 et noblesse −3, moral des recrues +5 | meat | — |
| `diet_dairy` | Laitages, fromages et œufs | terrain plaine, collines ou bocage | 0,12 | santé +4, croissance +3 % | dairy | fresh (×1,5) |
| `diet_wine_bread` | Pain blanc et vin | `res_wine` | 0,25 | richesse bourgeois +5, biens bourgeois +5, mécontentement bourgeois −3 | — | — |
| `diet_spiced_table` | Table épicée à la mode de Taillevent | `bld_market` ou `bld_fair` | 0,6 | prestige +1/saison, mécontentement noblesse −6, biens noblesse +8 | — | — |

Repère : une province rapporte ≈ 2,3 livres par millier d'habitants et par saison ; un hôtel-Dieu
coûte 20 livres/saison pour santé +8. Pour 300 000 habitants : fèves 24, laitages 36 (54 l'hiver),
poisson 45, vin 75, salaisons 105, table épicée 180 livres par saison.

## 4. Arbre Médecine

| Tier | Tech | Coût | Prérequis | Année (note) | Effets | Herbes |
|---|---|---|---|---|---|---|
| 1 | `tech_herb_garden` Jardin des simples | 120 | — | 800 ? (De Villis v. 795-800, plan de Saint-Gall) | santé +2 ; débloque `bld_herb_garden` | sauge, rue, menthe, fenouil |
| 1 | `tech_humoral_theory` Théorie des humeurs | 130 | — | 1025 ? (Canon d'Avicenne ; Hippocrate, Galien) | recherche +1 | — |
| 2 | `tech_regimen_sanitatis` Régime de santé de Salerne | 200 | humeurs | 1200 ? (XIIe-XIIIe s.) | `diet_health` +25 % | ail, oignon, hysope |
| 2 | `tech_willow_bark` Remèdes contre les fièvres | 180 | jardin | 1150 ? (Dioscoride ; Circa instans) | santé +3 | saule, reine-des-prés, camomille |
| 2 | `tech_barber_surgeons` Barbiers-chirurgiens | 200 | humeurs | 1268 (Livre des métiers) | blessés +15 % | plantain, consoude, millepertuis |
| 2 | `tech_hospital_reform` Réforme hospitalière (migrée) | 180 | — | 1300 | santé +5 ; débloque `bld_hotel_dieu` | — |
| 3 | `tech_theriac` Thériaque et apothicaires | 300 | fièvres | 1353 (ordonnance de Jean II) | santé +2, richesse bourgeois +3 ; débloque `bld_apothecary` | thériaque, aloès, safran |
| 3 | `tech_montpellier` Faculté de Montpellier | 320 | régime de Salerne | 1220 (statuts ; bulle de 1289) | recherche +2 | — |
| 3 | `tech_soporific_sponge` Éponge soporifique | 300 | barbiers | 1266 (Chirurgia de Théodoric Borgognoni, d'après Hugues de Lucques) | blessés +10 %, moral +2 | pavot, mandragore, jusquiame |
| 4 | `tech_plague_consilia` Conseils contre la peste | 420 | Montpellier | 1348 (Compendium de epidemia, Faculté de Paris) | peste +15 | genièvre, vinaigre |
| 4 | `tech_chauliac_surgery` Grande Chirurgie | 450 | éponge, Montpellier | 1363 (Chirurgia magna) | blessés +10 %, santé +2 | — |
| 4 | `tech_leprosaria` Léproseries et maladreries | 380 | réforme hospitalière | 1179 (Latran III ; legs de Louis VIII, 1226) | peste +10, mécontentement −2 | — |
| 4 | `tech_quarantine` Quarantaine (migrée) | 450 | assainissement urbain (civil) | 1377 (Raguse) | santé +6, croissance +3 %, peste +10 | — |
| 5 | `tech_aqua_vitae` Eau-de-vie des médecins | 700 | thériaque, Montpellier | 1351 ? (Roquetaillade ; Arnaud de Villeneuve) | santé +3, commerce +5 % | romarin (anachronisme signalé) |

« ? » = `uncertain: true`. Bâtiments : `bld_herb_garden` (sanitaire, tier 1, 400 l., 2 tours,
entretien 5 ; santé +3, recherche +0,5) ; `bld_apothecary` (sanitaire, tier 2, 1 200 l., 3 tours,
entretien 15, requiert `tech_theriac` et un marché ; santé +4, richesse bourgeois +4, peste +10).
Plafonds atteignables : peste 45 (3 techs + apothicairerie), blessés 35 %.

## 5. API du pont (`CampaignSim`)

Ordre (via `submit_order`) :

```gdscript
sim.submit_order({"type": "set_diet", "province": "prov_normandie", "diet": "diet_pulses"})
# -> {ok: false, error: "régime « Fèves, pois et lentilles » impossible à Normandie : technologie requise : Assolement triennal"}
```

Erreurs (français) : `régime inconnu : …`, `<Province> n'est pas contrôlée par votre faction`,
`le régime de <Province> a déjà été changé ce tour-ci`, `régime « … » impossible à <Province> :
<raisons séparées par « ; »>`.

| Méthode | Retour |
|---|---|
| `is_lent() -> bool` | Carême en cours (tour de printemps) |
| `get_province_diet(province) -> Dictionary` | `{diet, name, cost, changed_this_turn, lent_rule, winter_rule}` (`cost` : livres cette saison) |
| `get_province_diets() -> Dictionary` | `{province_id: diet_id}` pour toutes les provinces |
| `get_diet_options(province) -> Array` | `[{id, name, description, cost_per_thousand, cost, effects[{kind, value, mode, unit_category, class}], requirements{resources[], coastal, technology, any_building[], terrains[]}, lent_rule, winter_rule, available, reasons[], current, sources[]}]` ; `cost` effectif (hiver compris), `reasons` en français |
| `get_table_budget(faction) -> Dictionary` | `{total, last_turn, provinces[{province, diet, cost}]}` |
| `get_faction_economy(faction)` | + `table_upkeep` (projection) et `table_upkeep_last_turn` |
| `get_province_city(province)` | `effects` gagne `plague_resistance` |
| `get_tech_tree(faction)` | `branch` ∈ `military|civil|medicine` ; + `herbs[]` (ids codex), `historical_note` ; `effects[]` gagne `class` |
| `get_plague_resistance(province) -> float` | résistance à la peste en % (0-50) |
| `get_wound_recovery(faction) -> float` | part des pertes récupérée en % (0-50) |
| `get_events()` / `end_turn()` | nouveaux `kind` : `table`, `medicine` |

UI déjà touchée (correctif minimal) : onglet « Médecine » du panneau des technologies
(`tech_panel.tscn/.gd`, icône `tech_branch_medicine`) et comptage du smoke test. Icônes des 12
techs et 2 bâtiments ajoutées au catalogue (`tools/cent_ans_tools/icons_catalog.py`).

À faire par la vague UI : section « La Table » du panneau de province, bandeau Carême, libellés
des effets `plague_resistance`, `wound_recovery`, `diet_health` dans `rich_tooltip.gd`, genres
`table`/`medicine` dans le rapport de saison, Herbier.
