# JR4 — IA et équilibrage de la faction croisée

Spec `docs/superpowers/specs/2026-10-02-jr-croises-jerusalem-design.md` (§ 4.1 mis à jour), note
d'orchestration `docs/wip/jr-croises.md`. Worktree `../gp-jr` (`feat/jr`). État : TERMINÉ.

## Sonde
`core/crates/ai/tests/jr_crusade_probe.rs` (`#[ignore]`, 5 graines × 50 tours, ~45 s) :
`cargo test -p ai --test jr_crusade_probe -- --ignored --nocapture` ; `JR_SEEDS=1,4`,
`JR_TURNS=60`, `JR_TRACE=1` (armées, ordres, causes de ferveur), `JR_WATCH=fac_mamluks`.

## Ce qui bloquait l'IA
Au tour 1, l'IA croisée achetait la paix aux Mamelouks avec tout son trésor (8 000 livres), puis
licenciait faute d'argent ; en paix, −3/tour → ferveur 0 au tour 30 (3 graines sur 5). Les
revendications, les arêtes `sea` et la condition « province revendiquée » de l'IA marchaient déjà :
l'armée débarque dès le tour 1.

Correctifs (pilotés par `GameData.crusade_rules`, aucun id dans le Rust) :
- `crusade::ai_vow_forbids_peace` : menée par l'IA, la faction des règles ne propose ni n'accepte la
  paix avec le maître de la province cible (`negotiation` : blocage + `plan_peace` ; `diplomacy` :
  `evaluate` + ancien chemin). Le joueur reste libre (et paie −2/tour).
- `crusade::ai_passage_reserve` ajouté à la réserve du planificateur (`Context::reserve`) : le prix
  du passage est gardé quand la recharge finit.

## Règle
- Malus « frères de foi » seulement si les croisés attaquent (`on_battle(.., attacker)`).
- Crochet naval dans `naval::apply_outcome` (l'escadre qui intercepte attaque).
- Délivrance et perte de la cible : `WORLD_NEWS_MARK` (« délivrée », lu par
  `CRUSADE_PUBLIC_MARK` côté Godot) et `is_world_news` ; le texte de perte porte la marque.
- Nouveau `fervor.decay_above_high` (« L'exaltation retombe ») : −2 de plus par tour à ferveur
  ≥ `zeal.high_threshold`. Rend la jauge stable (plus de saturation à 99).
- Textes au pluriel accordé ; refus « Passage déjà prêché : nouvel appel dans 8 tours. ».
- Vue/pont : `zeal_high_threshold`, `zeal_low_threshold`, `zeal_high_morale`, `zeal_low_morale`,
  `desertion_threshold`, `desertion_men_percent`.

## Barème (JR1 → JR4)
usure 1 (+2 au-dessus de 70) ; victoire autre foi 6 → 3 ; défaite −8 → −6 ; place 10 → 4 ;
Jérusalem 40 → 25 ; prêche 10 → 8 ; passage recharge 6 → 8, unités 1 + f/25 (max 5) → 1 + f/40
(max 2) ; armée de départ 8 → 6 unités (chevaliers, sergents, piétons, 2 arbalétriers, milice) :
solde de départ joueur −156/saison à ferveur 60 (test `the_starting_host_is_about_balanced…`).

## Sonde finale (ferveur aux tours 10/20/30/40/50)
| graine | ferveur | min-max | trésor t50 | places t50 | débarq. | assiégées | Jérusalem | Mamelouks (prov. t50) |
|---|---|---|---|---|---|---|---|---|
| 1 | 64 70 70 66 66 | 54-73 | 1 812 | 7 | 4 | 5 | — | 11 |
| 2 | 70 72 73 78 64 | 62-97 | 2 044 | 6 | 7 | 7 | t31 | 10 |
| 3 | 68 70 93 75 67 | 61-96 | 1 408 | 3 | 3 | 6 | t28 | 11 |
| 4 | 74 72 91 69 59 | 59-97 | 257 | 3 | 3 | 5 | t28 | 11 |
| 5 | 68 66 69 75 63 | 61-94 | 3 085 | 3 | 2 | 6 | t31 | 11 |

Vivante partout, trésor négatif au plus 2 tours d'affilée, débarquement au tour 1 partout, sièges
en Terre sainte partout, Mamelouks à 11-12 provinces au tour 30 (aucun écrasement), jamais
détruite par eux.

## JR4b — cause côté Mamelouks
Diagnostic `core/crates/sim-campaign/tests/jr4b_budget_probe.rs` (`#[ignore]`) : recettes,
armée, garnisons, bâtiments, cour, solde structurel (hors part de la cour due au trésor qui dort)
de chaque faction au tour 1.
- Le « −4 000 » mamelouk est surtout la taxe d'opulence (20 % du trésor au-delà de 6 saisons de
  recettes : ~3 250) ; solde structurel −21 %. Le trésor (40 000) fond en 15 tours (opulence +
  dépenses de l'IA), puis le déficit fait licencier les armées de campagne en premier.
- Pas isolé : 152 factions sur 178 sous −15 %. Cause : coûts par colonie fixes (garnison de cité
  par rôle, garnison de départ par type, bâtiments) contre des recettes par tête ; les provinces
  OM sont vastes et peu peuplées (Mamelouks 110 000 hab./province contre 340 000 en France), et
  l'armée de départ codée (3 unités, 1 080/saison) dépasse les recettes des petites seigneuries.
- Correctif `settlements/rules.json` § `starting_budget` (`min_provinces` 5,
  `max_deficit_percent` 15) : une grande faction trop chère au départ renvoie ses unités de
  garnison les plus coûteuses, jamais la dernière d'une colonie ni la capitale. Effets (solde
  structurel) : Mamelouks −21 % → −14 %, Horde d'Or −80 % → −28 %, Byzance −42 % → −14 %,
  Mérinides −148 % → −90 %, Lituanie −148 % → −77 % (plancher atteint) ; France, Angleterre,
  Castille, Hongrie, Venise inchangées ; petites factions (< 5 provinces) inchangées.
- Insuffisant pour Jérusalem (3/5) : l'IA mamelouke reste sans armée de campagne. Ajout
  `crusade.json` § `relief` : quand la croisade assiège une place de Terre sainte, son maître y
  jette 2 unités (table pondérée), une fois par siège, recharge 12 tours, plafond de garnison ;
  évènement « Mamelouks appelle à défendre … ». Écart à la demande : la levée entre dans la place
  assiégée et non dans la province voisine, car un maître endetté licencie ses armées de campagne
  dès la saison suivante alors qu'une place assiégée garde sa garnison.

Sonde finale JR4b (ferveur t10-t50 ; Jérusalem) :
| graine | ferveur | min-max | trésor t50 | places t50 | Jérusalem | Mamelouks t50 |
|---|---|---|---|---|---|---|
| 1 | 60 60 44 42 61 | 35-71 | 1 913 | 9 | t50 | 10 |
| 2 | 58 58 50 40 38 | 36-67 | −137 | 4 | — | 12 |
| 3 | 54 54 46 46 36 | 36-67 | 995 | 4 | — | 12 |
| 4 | 60 64 56 62 52 | 52-69 | 232 | 4 | — | 12 |
| 5 | 68 50 42 44 30 | 26-73 | 631 | 5 | — | 12 |

## Points ouverts
- (Résolu par JR4b.) Jérusalem prise 4 graines sur 5 (tours 28-31) avant JR4b. Cause : les Mamelouks
  n'ont aucune armée de campagne après le tour 8 (déficit structurel : ~4 000 de recettes pour
  ~7 000 d'entretien au tour 1, l'IA licencie) ; Jérusalem (garnison de 2 milices, « intérieure »)
  tombe affamée après ~9 tours de siège sans secours. Essais : garnison de marche pour la cité du
  vœu (pire, 4/5 dès le tour 22, retiré), contingents plafonnés. Le levier est l'économie et la
  défense des Mamelouks (hors lot : données `fac_mamluks`, IA de secours), pas `crusade.json`.
- Grandes factions OM encore en déficit structurel après le correctif (Mérinides, Hafsides,
  Lituanie, Serbie) : leurs recettes ne couvrent pas même bâtiments + armée de départ ; à revoir
  avec les populations OM (lot historien) ou un coût de colonie proportionnel aux recettes.
- La ferveur tend à baisser après le tour 30 sans conquête (graine 5 : 26 au plus bas).
