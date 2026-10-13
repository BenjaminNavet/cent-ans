# Enquête faillites (inv/bankruptcy, 13/10)

État : enquête terminée ; correctifs 1, 3, 2 appliqués (voir § 8 « Correctifs appliqués »), 4 et 5 non faits. Sonde : `core/crates/ai/examples/bankruptcy_probe.rs`
(commit `tools(ai): bankruptcy probe`). Prochaine étape : le joueur choisit parmi les propositions ci-dessous.

## 1. Ce qu'est une « faillite » dans le code

- `EventKind::Bankruptcy` est émis par `resolve_economy` (`core/crates/sim-campaign/src/economy.rs`, phase 5) **pour toute
  faction vivante (hors rebelles) dont le trésor est < 0 une fois la paie du tour passée** (revenus − entretien).
  Ce n'est donc pas un événement de bascule mais un **état compté à chaque tour** : une faction qui reste 12 tours à découvert
  produit 12 « faillites ».
- Conséquences : moral −10 (`bankruptcy_morale_penalty`) sur armées et garnisons, `unpaid_seasons` +1 par armée ;
  à 2 saisons d'impayés (`desertion_after_unpaid_seasons`) 5 % des hommes désertent par saison (`data/rules/economy.json`).
  Côté IA : `relieve_debt` (`core/crates/ai/src/campaign/economy.rs`) licencie les unités les plus coûteuses pour rembourser
  la dette en `debt_repayment_turns` = 8 tours.
- La métrique seed-bots `bankruptcies_per_game` compte ces événements (`seed-bots/src/metrics.rs`), toutes factions de la carte
  (~145 vivantes). Rapporté à ~22 000 faction-tours, 100-290 événements = 0,5-1,3 %.
  Important : le trésor est contrôlé **avant** la chronique (phase 8c) ; un découvert né de la chronique n'est compté
  que s'il survit à la paie du tour suivant. Les événements comptés sont donc déjà les découverts « qui durent ».

## 2. Méthode

Sonde `bankruptcy_probe [graine] [tours] [faction joueur] [csv] [faction à tracer]` : une partie complète pilotée par l'IA
(le « joueur » = France jouée par `ai::plan_turn`), profil `seed-bots`, 150 tours, graines 1 et 2 (≈ 12 min chacune).
Elle écrit un CSV par événement (budget de la faction) et, sur stdout, bilan par faction, épisodes de trésor négatif en fin de
tour (état, indépendant des événements), cause de l'entrée en découvert (trésor avant, solde du budget, « autres flux »),
ordres de l'IA du tour. Attribution par phase : instrumentation **temporaire** (non commitée) de `resolve_end_of_turn`
mesurant le delta de trésor par phase ; puis instrumentation temporaire de `chronicle::apply_option` journalisant chaque
effet qui baisse le trésor (5 379 lignes, graine 1).

## 3. Chiffres

| | graine 1 | graine 2 |
|---|---|---|
| événements Bankruptcy | 105 | 79 |
| factions distinctes touchées | 22 / ~147 | 19 / ~143 |
| joueur (France) concerné | 0 | 0 |
| « épisodes » (tours consécutifs d'une faction) | 42 : 22 d'1 tour, 6 de 2, 10 de 3-5, 4 de 6+ | 31 : 12 / 9 / 7 / 3 |
| part des 3 pires factions | 43 % (Hongrie 21, Byzance 16, Teutoniques 8) | 47 % (Byzance 16, Marinides 11, Teutoniques 10) |
| factions à 1 seul événement | 7 sur 22 | 5 sur 19 |
| tours-factions en trésor négatif (fin de tour) | 297 (142 entrées) | 257 (133 entrées) |

- **Répétition** : oui mais concentrée : ~45 % des événements viennent de 3 factions (Byzance, Hongrie, Marinides, Teutoniques
  reviennent d'une graine à l'autre) ; la majorité des factions touchées l'est 1 à 3 fois. Les faillites ne sont donc ni un
  phénomène « toutes les factions tout le temps » ni 2-3 factions seules.
- **Qui** : 100 % IA, jamais le joueur (France IA incluse). Taille : 40-50 % des événements touchent des factions de 10 provinces
  et plus (revenu moyen 2 000-2 700 ₶/saison), 15-20 % des factions à 1 province (comtés : Alençon, Armagnac, Pise…), le reste 2-9 provinces.
  Ce ne sont donc pas surtout les petites factions féodales. Période : 3-6 % avant le tour 20, 33-45 % tours 20-50 (première vague
  d'événements historiques), 32-55 % tours 50-100, 13-16 % après.
- **Pas structurel** : sur les lignes d'événements, revenu < entretien dans 3 cas sur 105 (graine 1) et 1 sur 79 ; `deficit_seasons` > 0 dans 3 et 1.
  Les 62 + 37 lignes avec 0 armée (armée licenciée/désertée) montrent la suite du choc : l'IA a perdu ses troupes.
  Point secondaire : les bâtiments coûtent plus que les armées dans 65 % des lignes (Byzance : entretien bâtiments 1 767 + administration 798
  pour 4 342 de revenu, marge ~0), mais la marge existe.

## 4. Causes (classées)

Entrées en découvert (graine 1, 142 entrées, attribution par phase de tour) :

1. **Paiements d'événements de la chronique (aléatoires et historiques) : 108 entrées sur 142 par phase dominante (76 %), 130 sur 142
   (92 %) en comptant les effets mesurés dans `apply_option`**, pour 84 270 ₶ au total. Les 130 passages de ≥ 0 à < 0 sont **tous**
   des choix de l'IA dont le coût dépasse le trésor (130/130) mais reste ≤ trésor + 2 × revenu : la règle d'abordabilité de
   `ai_affordable_choice_among` (`core/crates/sim-campaign/src/chronicle.rs`, ~l. 640) accepte l'endettement par construction.
   Événements les plus fréquents : `evt_defection_capitaine` (17, 2 500 ₶), `evt_incendie` (13), `evt_brigands` (12), `evt_medecin_de_montpellier` (9),
   `evt_miracle_local` (9), `evt_pirates` (8), `evt_pelerinage` (8), `evt_crue` (7), `evt_vassal_mecontent` (7). 94 événements de `data/events/` ont un coût en trésor
   (34 aléatoires à 50-150 tours de délai moyen, la plupart à 1 000-2 500 ₶ ; les 50 historiques coûtent 1 000-12 000 ₶). L'échelle par revenu
   (`event_treasury_reference_income` 4000, `event_treasury_min_scale` 0,03, `data/rules/economy.json`) atténue les petites factions mais
   un royaume à 1 100 ₶/saison de revenu paie encore ~27 % du plein tarif, soit près d'un trimestre de revenu.
   Médiane du coût d'une entrée en découvert : 30 % d'une saison de revenu.
2. **Prix forcés** (32 entrées de la phase diplomatie, 22 « tribut/traité » dans la sonde) : `pay_sale_price` (`event_actions.rs`, l. 85-100)
   débite sans contrôle. Cas typique graine 1 t38 : l'Ordre Teutonique (trésor 699, revenu 1 100) paie 6 000 ₶ (`evt_vente_de_l_estonie`)
   → −5 136 ₶, et `relieve_debt` (rembourser en 8 tours = 640 ₶/tour) licencie toute l'armée (312 → 0 d'entretien en 2 tours) pour une dette
   qui se résorbe seule en 8 tours. Les traités de paix de guerre perdue (tributs) alimentent aussi (Croisés, Durazzo).
3. **Dépenses d'ordres de l'IA** (recrutement, bâtiments, agents) : marginal. Les 2 seules entrées du tour 1 (Poméranie, Tarente) dépensent
   2 100-2 600 ₶ en construction/recrutement/acte royal sur 2 300-2 650 de trésor initial avec une marge de revenu de +10 : le budget IA ne laisse pas de
   réserve (`reserve()`), mais c'est 2 cas sur 142.
4. **Déficit structurel** : 1-3 cas par partie ; l'IA licencie (`relieve_debt`) et démolit, le mécanisme fonctionne.
5. **Rançons** : 1 entrée.

Rattrapage : il en existe (licenciements, démolition, dons suspendus, éditos abandonnés), mais **aucun garde-fou en amont sur les
paiements forcés ou choisis par événement**, et le rattrapage lui-même sur-réagit à un choc ponctuel (licencier toute l'armée
pour une dette de 5 600 ₶ sur 8 tours alors que le revenu net est positif).

## 5. Cible

Total War / CK : un découvert est un état exceptionnel et visible (au plus quelques factions par campagne, durable, causé par la
guerre ou le tribut). Ici : 0,5-1 % des faction-tours, ~20 factions touchées par partie, 10-14 épisodes ≥ 3 tours, 3-4 de 6+ tours. Proposition de cible
(à intégrer dans les bandes de `data/balance/` quand tranchée) : ≤ 15 événements `Bankruptcy` par partie de 150 tours, ≤ 3 épisodes de 6 tours
ou plus, aucune faction ≥ 5 provinces en découvert hors tribut de guerre.

## 6. Propositions de réglage (non appliquées)

1. **Abordabilité stricte des choix d'événements par l'IA** (règle, `chronicle.rs::ai_affordable_choice_among`) : `means` = trésor − réserve de sécurité
   (au plus un demi-revenu) au lieu de trésor + 2 × revenu ; si aucune option n'est abordable, prendre la moins chère (et non `weighted_pick` sur toutes).
   Effet attendu : supprime l'essentiel des 130 entrées (92 %) ; coût en jeu : l'IA subit plus souvent l'option gratuite (désagrément diplomatique/prestige).
   À vérifier avec la passe seed bots (bandes `bankruptcies_per_game`, `revolts`, `war_turns_mean`).
2. **Plafonner les paiements d'événements** (donnée + une règle simple) : ajouter à `data/rules/economy.json` un plafond
   `event_treasury_cap_seasons` (p. ex. 1,5 saison de revenu) appliqué dans `effects.rs::event_treasury_amount`, et abaisser
   `event_treasury_reference_income` 4000 → 2500. Effet : un royaume à 1 100 ₶ paie au plus ~1 650 ₶ et plus 6 000 ₶ ; les gros royaumes ne changent pas.
   Alternative sans code : réduire les montants de `data/events/evt_defection_capitaine.json`, `evt_incendie.json`, `evt_brigands.json`.
3. **Prix forcés conditionnés** (`event_actions.rs::pay_sale_price`) : si le payeur ne peut pas (trésor < prix), le transfert devient un
   paiement échelonné (tribut sur N saisons, mécanisme `ledger.tributes` existant) ou l'option est refusée. Effet : supprime les cas à −5 000 ₶ et
   la désertion en cascade (Teutoniques, Croisés, Durazzo).
4. **Adoucir le rattrapage** (`data/ai/campaign.json` : `debt_repayment_turns` 8 → 16, éventuellement `max_disbands_per_turn` 12 → 4) et, si
   le plafond des propositions 1-3 est retenu, mesurer en passe longue ; sinon mesurer d'abord. Effet : l'IA ne licencie plus toute son armée pour un découvert ponctuel.
5. **Métrique** (ADR 0348, hors équilibre) : compter les *entrées* en faillite (bascule ≥ 0 → < 0) plus les épisodes de 3 tours et plus, au lieu de tours en découvert,
   pour que la bande soit lisible (« 100 à 290 » mélange nombre de factions et durée).

Ordre conseillé : 1 puis mesure (une passe `--quick` suffit : le mécanisme est direct), 3, puis 2 si le résultat reste au-dessus de la cible.

## 7. Limites

Deux graines seulement (1, 2) sur 150 tours ; attribution par phase mesurée sur la graine 1 uniquement ; le « joueur » est piloté par l'IA, ce
qui exclut les décisions humaines sur les événements (le joueur voit un message « Trésor insuffisant » et ne peut pas choisir une option qu'il ne peut payer : `option_unavailable`).

## 8. Correctifs appliqués (branche `fix/bankruptcy`, 13/10)

Ordre du plan validé : 1, mesure, 3, mesure, 2 (parce que « bascules » restait > 15), mesure. Sonde `bankruptcy_probe`, graines 1 et 2, 150 tours.
« Bascules » = épisodes de trésor négatif en fin de tour (sortie `episodes`) ; « événements » = `EventKind::Bankruptcy`.

| Étape | Commit | Événements (g1 / g2) | Bascules (g1 / g2) | Épisodes ≥ 3 t (g1 / g2) | Épisodes ≥ 6 t (g1 / g2) |
|---|---|---|---|---|---|
| Avant | (enquête) | 105 / 79 | 142 / 133 | 14 / 10 | 4 / 3 |
| 1. Réserve IA (`ai_event_reserve_seasons` = 1,0) | 0d475c88d | 13 / 15 | 22 / 42 | 5 / 6 | 1 / 1 |
| 3. Vente forcée annulée si fonds insuffisants | 8db96b2d8 | 14 / 6 | 36 / 46 | 8 / 2 | 4 / 0 |
| 2. Plafond 1,5 saison + référence 2500 | cb8044465 | 12 / 18 | 36 / 41 | 7 / 6 | 2 / 5 |

Lecture :
- Le gros du gain vient de l'étape 1 : événements Bankruptcy ÷ 7 à ÷ 8 (92 → ~14 par partie), cible « ≤ 15 événements » atteinte dès l'étape 1.
- Les étapes 3 et 2 ne se distinguent pas du bruit : une modification quelconque décale toute la trajectoire de la partie (les graines divergent), d'où 6-18 événements d'une mesure à l'autre. Deux graines ne suffisent pas à trancher leur effet ; elles sont gardées car la logique est directe (plus de prix forcé à −5 000 ₶, coût plafonné) et réglable par donnée (`event_treasury_cap_seasons` à 0 et référence à 4000 rétablissent l'ancien comportement).
- La cible « ≤ 15 bascules » **n'est pas atteinte** (36-46) : ces bascules sont presque toutes des dépassements de quelques livres (médiane 1 tour, trésors de −1 à −50 ₶) qui ne produisent pas d'événement Bankruptcy ; causes restantes de la sonde : « treaty payment » (14-21, indemnités et tributs de la phase diplomatie, non traités ici) et « other/spending » (17-19, dépenses d'ordres de l'IA sur de petites factions à 1-3 provinces et revenu quasi nul). Ce sont les propositions 4 (rattrapage) et 5 (métrique d'entrées) et le paiement des traités qui restent à décider.
- Épisodes de 6 tours et plus : 3-4 par graine avant, 0-5 après (bruit) ; plus aucun royaume de 10 provinces et plus en découvert durable côté événements (Byzance, Hongrie, Teutoniques ne reviennent plus en tête).

Choix : réserve = 1 saison du dernier revenu ; vente forcée annulée plutôt qu'échelonnée par `ledger.tributes` (ADR 0350). Tests de régression :
`economy/ai_event_reserve.rs` (réserve, plafond) et `feudal/lr17_sales.rs::a_sale_the_payer_cannot_afford_is_cancelled`.
