# M10 (partie 1) — Événements historiques et aléatoires : spécification

Date : 2026-09-23. Objectif : un moteur d'événements piloté par les données, qui fait vivre la chronique de
la guerre de Cent Ans (grands événements datés, conditionnels) et des événements aléatoires à choix, avec
une fenêtre de décision pour le joueur. Design général : `docs/design/2026-09-23-cent-ans-design.md` § 5 (M10).

## 1. Données (`data/events/*.json`, `data/schemas/event.schema.json`)
Chaque événement : `id`, `title` (fr), `text` (fr, 2-4 phrases d'époque), `kind` (`historical` | `random`),
`trigger` :
- `date` (année, saison optionnelle) pour les historiques, ou `mean_time_to_happen` (tours) / `chance_permille`
  par tour pour les aléatoires ;
- `conditions` : liste de conditions typées (`faction_exists`, `faction_is_player`, `at_war {a, b}`,
  `controls {faction, province}`, `character_alive {id}`, `ruler_is {faction, character}`, `year_between`,
  `province_unrest_above`, `treasury_above {faction, amount}`, `religion_is`, `schism`, `not_fired {event}`,
  `ruler_trait {trait}`, `season`) ;
- `scope` : `global`, `faction` (une faction ciblée ou chaque faction qui remplit les conditions),
  `province` (tirée parmi celles qui remplissent) ;
- `options` : 1 à 3 choix, chacun avec `text` (fr), `effects` (liste typée : `treasury {amount}`,
  `unrest {province|all, amount}`, `population {province|all, percent}`, `health {…}`, `prestige {character|ruler,
  amount}`, `piety`, `papal_favor`, `opinion {faction, amount, reason}`, `declare_war {a, b}`, `peace {a, b}`,
  `add_trait {character, trait}`, `kill_character {id}`, `spawn_army {faction, province, units[]}`,
  `devastation {province, amount}`, `claim {faction, kind, target}`, `loyalty {vassal, amount}`) et
  `ai_weight` (pour le choix de l'IA).
Au moins 40 événements, sourcés (`sources`), dont une vingtaine d'historiques : Crécy/siège de Calais (1346-47,
si Angleterre et France en guerre), Peste noire (1347-1351, vague qui frappe la santé et la population de
toutes les provinces en progressant du sud vers le nord), traité de Brétigny (1360, si Jean II captif),
Jacquerie (1358), Étienne Marcel à Paris (1357-58), Grandes Compagnies (1360-1370), révolte des Paysans
anglais (1381), révolte des Ciompi (1378), les Maillotins (1382), folie de Charles VI (1392, si vivant et roi),
guerre civile Armagnacs et Bourguignons (1407+), Azincourt (1415), traité de Troyes (1420), Jeanne d'Arc
(1429, condition : Orléans assiégée ou France en difficulté), concile de Constance (lien avec le Schisme,
sans doublon avec M5), bataille de Castillon (1453). Une vingtaine d'aléatoires : bonne récolte, disette,
incendie de ville, foire prospère, pèlerinage, scandale à la cour, brigands, crue, épidémie locale,
don d'un marchand, défection d'un capitaine, miracle local, querelle de préséance…
Les historiques ne se déclenchent que si leurs conditions tiennent (l'histoire peut diverger) ; ils ne se
déclenchent qu'une fois.

## 2. Simulation (`sim-campaign`, nouveau `chronicle.rs`)
- `CampaignState` gagne `fired_events: BTreeSet<EventId>`, `pending_decisions: Vec<Decision{id, event,
  faction, province?, options[], expires_turn}>`. `state_version` : ne pas changer la valeur (4) si le merge
  se fait avant la sortie ; utiliser `#[serde(default)]` pour tous les nouveaux champs.
- Phase de fin de tour (après la religion, avant la population) : évalue les événements, tire les aléatoires
  avec le RNG de campagne (déterministe), applique immédiatement le choix de l'IA (option de plus fort
  `ai_weight`, départage par RNG) ; pour le joueur, crée une `Decision` (le premier choix s'applique
  d'office si elle expire au bout de 2 tours). Événements de journal `chronicle` (texte fr).
- Ordre `choose_event_option { decision, option }`.
- Effets appliqués via une fonction unique `apply_effect` validée ; un effet invalide (id inconnu) est ignoré
  avec un avertissement au chargement (validation dans `data-model`).
- La Peste noire : effet dédié `plague_wave { from_year, to_year }` qui touche les provinces par latitude
  (sud d'abord) sur ~12 tours : santé -30, population -15 à -30 %, mécontentement +20.
- Tests (≥ 10) : déclenchement par date, conditions non remplies (histoire divergente), une seule fois,
  aléatoire déterministe, choix IA, décision joueur puis ordre, expiration, peste noire, sauvegarde, validation
  des données (tous les événements chargent).

## 3. Pont et interface
- `CampaignSim.get_pending_decisions() -> [{id, title, text, options[{index, text, effects_text}], expires_in,
  province}]`, ordre `choose_event_option`.
- Godot : fenêtre « Chronique » parchemin (titre, texte, boutons d'options avec info-bulle des effets),
  ouverte en fin de tour quand une décision attend ; file si plusieurs. Nouvelle scène/script sous
  `game/scenes/ui/` et `game/scripts/ui/`, branchement minimal dans `campaign_map.gd` (bloc marqué M10).
- Journal : couleur dédiée `chronicle`.
- Smoke : 60 tours France, au moins un événement historique et un aléatoire, une décision résolue par ordre.
- Capture : `docs/img/godot-chronicle.png` (`--stage=chronicle`).

## 4. Critères de fin
Tests verts, smoke vert, capture relue, `docs/status.md`, `docs/godot-map.md`, `docs/design/data-model.md`
à jour.

## F1 (v2) — capture, chaînes, Charles VI
Nouveaux effets : `capture_character { id, faction?, captor }`, `release_character { id, faction?,
ransom }` (rançon versée au geôlier), `schedule_event { event, delay }` (≥ 1 tour) et `marry { a, b }`.
Nouvelle catégorie `chained` : l'événement ne part que programmé, au tour N + k, pour la même faction et la
même province, conditions vérifiées alors (`ChronicleState::scheduled`). Validation : délai nul ou
auto-programmation refusés, cible inconnue ou événement chaîné jamais programmé signalés. Poitiers capture
Jean II ; les États programment « La rançon du roi Jean » ; Brétigny (captivité de Jean ou, en repli,
Poitiers) le libère contre 40 000 livres s'il est encore captif. « Les noces du dauphin » (1350) marient
Charles et Jeanne de Bourbon ; Charles VI naît en 1368 et sa folie le vise (il règne, 18-40 ans).

## F7b — chronique enrichie
40 nouveaux événements (total 90 : 54 historiques, 8 chaînés, 28 aléatoires). Déjà présents, donc non
dupliqués : L'Écluse, Nicopolis, du Guesclin connétable, Cabochiens, mort du Prince Noir, assassinat de Louis
d'Orléans (`evt_armagnacs_bourguignons`), Troyes, Jeanne d'Arc ; le Grand Schisme relève de la religion (M5).
- Historiques (27) : `evt_paix_de_venise` (1339), `evt_laupen` (1339), `evt_valdemar_iv` (1340),
  `evt_siege_tournai` (1340), `evt_salado` (1340), `evt_succession_bretagne` (1341), `evt_banqueroute_bardi`
  (1343-1346), `evt_charles_iv_roi_des_romains` (1346), `evt_neville_cross` (1346), `evt_cola_di_rienzo` (1347),
  `evt_achat_dauphine` (1349), `evt_combat_des_trente` (1351), `evt_bulle_d_or` (1356), `evt_cocherel` (1364),
  `evt_auray` (1364), `evt_najera` (1367), `evt_appel_gascon` (1369), `evt_auld_alliance` (1371),
  `evt_la_rochelle` (1372), `evt_lords_appelants` (1387), `evt_harfleur` (1415), `evt_montereau` (1419),
  `evt_verneuil` (1424), `evt_patay` (1429), `evt_proces_de_rouen` (1431), `evt_fougeres` (1449),
  `evt_formigny` (1450).
- Chaînes (`schedule_event`, 7 étapes) : Tournai → `evt_treve_esplechin` ; succession de Bretagne →
  `evt_hennebont` ; Neville's Cross (capture de David II) → `evt_rancon_david_ii` (libération contre rançon) ;
  Rienzo → `evt_chute_de_rienzo` ; Nicopolis (option « Négocier ») → `evt_rancon_de_nevers` ; Montereau →
  `evt_alliance_anglo_bourguignonne` → Troyes (condition élargie) ; Jeanne d'Arc → `evt_sacre_de_reims`.
- Aléatoires des factions F7 (6) : `evt_galeres_de_flandre` (Venise), `evt_banque_florentine`,
  `evt_piquiers_suisses`, `evt_argent_de_kutna_hora` (Bohême), `evt_harengs_de_scanie` (Suède),
  `evt_razzia_frontiere` (Grenade, en guerre avec la Castille).
- Reprises de guerre : dans une campagne simulée à l'IA minimale, la guerre franco-anglaise s'éteint vers
  1355. `evt_appel_gascon` (1369), `evt_harfleur` (1415) et `evt_fougeres` (1449) la redéclarent (option IA),
  ce qui rouvre La Rochelle, Azincourt, Troyes, Jeanne d'Arc, Formigny, Castillon. Les conditions des
  événements tardifs reposent sur l'existence des factions et « ruler »/« heir » (les personnages après 1400
  sont générés par la dynastie).
- Modifications légères : `evt_nicopolis` (première option programmée), `evt_troyes` (alliance
  anglo-bourguignonne comme alternative à Azincourt + Normandie), `evt_jeanne_d_arc` (possible après Troyes
  même en paix ; programme le sacre).
- Test `core/crates/sim-campaign/tests/f7_events.rs` : campagne 1337-1453 (graine 1337, France jouée par
  l'IA) où ≥ 20 nouveaux historiques et ≥ 4 étapes chaînées doivent se déclencher (27/27 et 7/7 obtenus ;
  mêmes résultats pour Angleterre/7 et Bourgogne/42 via le test ignoré `print_campaign_chronicle`).
- Tirage aléatoire (`chronicle.rs`, `scope_allows`) : un événement aléatoire réservé à une autre faction ne
  consomme plus de tirage du RNG pour les autres factions (ajouter un événement vénitien ne rebat plus les
  cartes de la France ; sans cela le smoke `flow` perdait son rapport de saison, sensible à la graine).

## G1 — `transfer_province`
Effet `transfer_province { province, faction?, from? }` : la province passe (propriété et contrôle) à
`faction` (défaut : la faction qui décide) ; siège, garnison, file de recrutement, chantier et gouverneur de
l'ancien détenteur prennent fin (même code que la cession d'une rançon). Sans effet si `from` ne la possède
ni ne la contrôle, si le destinataire est mort ou la tient déjà, ou si c'est la capitale de son propriétaire.
Validation : province et factions inconnues signalées. Utilisé par l'achat du Dauphiné (1349, `from`
Empire, remplace la prétention), le traité de Guérande (`evt_auray`, option Montfort : les deux provinces
bretonnes rendues au duc) et Formigny (option « Abandonner la Normandie » : les provinces normandes tenues
par l'Angleterre passent à la France). Écarts : `evt_valdemar_iv` (pas de faction Danemark), la paix de
Venise (Trévise n'est pas une province).

## LR-17 — ventes, cessions de titres et changements de souverain
- `transfer_province` gagne `price` (livres, défaut 0) et `payer` (défaut : l'acquéreur) : le prix n'est
  versé, à l'ancien propriétaire, que si la province change de mains.
- `transfer_title { title, faction?, from?, price?, payer? }` : le titre passe par le transfert féodal
  (F3) ; ses provinces de jure tenues par le vendeur suivent, **capitale comprise** ; un vendeur laissé
  sans titre se fond dans l'acquéreur (terres, armées, cour, trésor). Avec `from`, seulement si cette
  faction tient le titre. Le prix est payé (même à découvert) au vendeur s'il subsiste, sinon il quitte
  la carte (couronne non jouable, vendeur absorbé). L'IA compte le prix dans le coût d'une option quand
  la faction qui décide paie.
- `set_ruler { character, faction? }` : un personnage vivant de la faction prend le trône ; l'ancien
  souverain vit (déposé) ; l'héritier est recalculé par la loi de succession. Sans effet pour un mort,
  un personnage d'une autre faction ou le souverain en place.
- Événements : `evt_vente_de_l_estonie` (été 1346, l'Estonie danoise décide ; la vente donne le titre à
  l'ordre Livonien, payée 6 000 ₶ par l'ordre Teutonique, et fait disparaître la faction),
  `evt_algirdas_grand_duc` (printemps 1345, si Jaunutis règne), `evt_election_d_osel_wiek` (printemps
  1338, si Jakob II règne encore ; nouveau personnage `chr_hermann_ii_osenbrugge`). Test :
  `core/crates/sim-campaign/tests/lr17_sales.rs`.
