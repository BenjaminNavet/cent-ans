# WH — diplomatie et IA de campagne vs TWW3

Chemins relatifs à `core/crates/sim-campaign/src/` (« sc/ »), `core/crates/ai/src/` (« ai/ »), `game/scripts/ui/` (« ui/ »), `data/`.

## 1. Résumé

- Le socle ressemble déjà beaucoup à TWW3 et le dépasse sur deux points : un traité à articles (19 sortes, `negotiation.rs:92`) avec jauge d'acceptation chiffrée, motifs ligne à ligne, contre-offre sur un point bloquant (ADR 0202, 0182, `treaty_explain.rs`) ; une carte diplomatique à 7 positions et une vue « du point de vue de la faction choisie » (`stance.rs`, `diplomacy_map_view.gd`).
- La guerre a ses conséquences (parjure −40, agression sans motif −20, prestige, excommunication, appel aux armes, trêves liant les co-belligérants) et son aperçu d'escalade (`war.rs:12`, `campaign_sim_diplomacy.rs:233`).
- Ce qui manque le plus : les **clauses d'action tierces** (rejoindre une guerre, déclarer la guerre à X, rompre avec Y), le **pacte de non-agression** et la distinction **alliance défensive / militaire**, les **demandes de soutien** (le joueur rejoint les guerres de ses alliés d'office, sans choix : `diplomacy/ai.rs:192`), la **durée et la révocation** des pactes (l'accès militaire est irrévocable), et la **lisibilité de l'opinion** (pas de durée des modificateurs, pas d'étiquette qualitative, pas de prévision de réaction des alliés à une déclaration de guerre).
- L'IA ne propose au joueur que paix, alliance, commerce, accès/passage et mariage ; jamais demande d'aide, ultimatum, tribut ni cadeau. Pas de coalition anti-hégémonique (déjà signalée dans RX, non traitée).
- Déjà RX (non reproposé) : guerre permanente (≈ 3 déclarations/tour), annexions totales sans coût, cadence des trêves — `rx/mecaniques.md` lots `equil` (0257-0258, en attente). Bouton Diplomatie / pastille : `rx/ui.md` lot `uifin`.

## 2. Tableau des écarts

| # | écart vs TWW3 | état actuel | impact | coût | proposition | dépend. |
|---|---|---|---|---|---|---|
| 1 | Clause « Rejoindre la guerre contre X » / « Déclarer la guerre à X » (outil central de coordination TWW3) | Absente : 19 articles, aucun ne désigne un tiers (`negotiation.rs:92-186`). L'IA rejoint seule via `ally_war_to_join` (`diplomacy/ai.rs:453`), non pilotable | 5 | M | `Article::JoinWar { giver, target }` : check (cible pas allié du giver, pas de trêve), value (reprendre `join_war` + `alliance_factors` `value.rs:189`), apply (`declare_war` côté giver), label ; UI : entrée du menu « Vous demandez » | — |
| 2 | Alliance défensive vs militaire, pacte de non-agression | Un seul `Article::Alliance` (`negotiation.rs:97`) ; appel aux armes systématique dans les deux sens (`war.rs:146`) ; pas de pacte de non-agression | 4 | M | `Article::NonAggression { turns }` (interdit `declare_war` entre les deux, parjure si rompu, + attitude) ; `Article::Alliance` devient `Alliance { kind: Defensive|Military }` : défensive = n'appelle que si la cible est attaquée, militaire = aussi les guerres offensives de l'allié (`ally_war_to_join`) | 1 |
| 3 | Demande de soutien d'un allié (joueur) | Le joueur allié rejoint **d'office** (`diplomacy/ai.rs:192` renvoie `true`) ; aucun refus possible, aucun message de choix | 4 | M | Pour une alliée IA attaquée et un joueur allié : créer une `Offer` `Article::AllyCall { aggressor }` (modèle `Protection` féodal, `negotiation.rs:160`) : accepter = `start_war`, refuser/expirer = modificateur −30 « A refusé l'appel » et alliance rompue comme pour l'IA (`war.rs:176-200`). Test : appel de l'IA, refus, expiration | — |
| 4 | Demande d'aide du joueur à ses alliés pendant ses guerres, et message d'honneur quand il ne vient pas | Le joueur ne peut qu'espérer que `ally_war_to_join` (tous les 2 tours, ratio 0,6) s'active ; aucun retour sur les raisons du refus | 4 | S (après #1) | Le #1 couvre l'action ; ajouter dans `treaty_explain.rs` les raisons d'un refus de `JoinWar` (fatigue, trésor, rival commun) | 1 |
| 5 | Pactes sans durée ni révocation | `military_access` n'a aucun ordre de retrait (grep : seuls `apply.rs:67`, `negotiation.rs:1080` à la guerre) ; commerce rompable seulement (`ties.rs:69`) ; pas d'échéance | 3 | S | `Order::RevokeMilitaryAccess { target }` (modificateur −15 « Accès retiré », 20 tours), bouton dans `diplomacy_actions_section.gd:36` ; option `turns` sur `MilitaryAccess` et `TradeAgreement` (5 ans, renouvelables) | — |
| 6 | Détail de l'attitude : durée et origine des modificateurs | `attitude()` (`attitude.rs:6`) renvoie (texte, valeur) ; `OpinionModifier.expires_turn` existe (`mod.rs:98`) mais n'est pas exposé ; l'UI ne montre les motifs qu'en infobulle (`diplomacy_faction_list.gd:80`) | 3 | S | Étendre le pont `get_diplomacy` : `attitude_reasons[i].turns_left` ; section repliable « Opinion » dans `diplomacy_head_section.gd` groupant par catégorie (Relations, Foi, Guerre, Parole donnée, Souverain) avec « encore N tours » | — |
| 7 | Étiquette qualitative de l'attitude | Seulement « attitude +N » (`diplomacy_head_section.gd:27`, liste `:104`) | 2 | S | `attitude_band(n) -> &str` dans `sc/diplomacy/attitude.rs` (Hostile < −50, Défiant, Réservé, Cordial, Amical, Dévoué), table dans `data/rules/diplomacy.json`, affichée avec le nombre | — |
| 8 | Prévision de la réaction des alliés à une déclaration de guerre | `war_verdict` liste « Alliés appelés aux armes » sans dire s'ils viendront (`campaign_sim_diplomacy.rs:253`) alors que `answers_call_to_arms` est pure (`ai.rs:172`) | 4 | S | Pour chaque allié de la cible, ajouter « viendra / hésite / refusera » (appel de `answers_call_to_arms`), le coût en prestige de la déclaration (−20/−30 `war.rs:36-53`) et l'effet d'une rupture de trêve sur l'opinion | — |
| 9 | Coalitions contre l'hégémon | Aucune : grep « coalition/ligue » ne trouve que la puissance agrégée (`plan_cache.rs:88`). Seule trace : la correction proposée `rx/mecaniques.md` (« ligue anti-hégémon »), sans lot | 4 | M | Voir spec 5 (« Ligue anti-hégémon ») | RX equil |
| 10 | L'IA ne propose rien d'actif au joueur hors paix/alliance/commerce/accès | Offres IA → joueur : `create_offer` (`offers.rs:30`) ; sources : `plan_peace`, `plan_alliances` (`ai.rs:501`), `plan_treaties` (`diplomacy_eval.rs:36`, commerce, accès, passage), mariage (`ai/campaign/characters.rs:178`). Pas de demande d'aide, d'ultimatum, de tribut, de vassalité proposée | 3 | M | Après #1/#3 : l'IA envoie des `JoinWar` à ses alliés (dont le joueur) quand elle est en guerre ; ultimatum « cédez X ou guerre » via l'article `CedeProvince` + `DeclareWar` différé à 2 tours | 1, 3 |
| 11 | Carte diplomatique : relations entre tiers | Statuts ne montrent que X↔joueur (liste `diplomacy_faction_list.gd`) ; la carte permet la vue d'une faction choisie (DZ) mais pas la liste de ses alliances/guerres/accords dans sa fiche | 3 | S | Dans la fiche : « Alliés : … · En guerre contre : … · Vassaux : … » depuis `get_diplomacy` pour la faction choisie (données déjà dans `FactionState.allies/at_war_with`) | — |
| 12 | Confédération / absorption d'un allié | Vassalité (`Article::Vassalage`) ; héritage féodal (`feudal/inherit.rs`) ; pas de fusion volontaire de deux factions | 2 | L | Voir refonte A | — |
| 13 | Historique des traités : pas de rupture visible | `diplomacy_history_tab.gd` ne liste que les traités négociés (signé/refusé) ; ruptures et parjures ne sont que des modificateurs | 2 | S | Enregistrer ruptures d'alliance/accès/commerce et appels refusés dans `TreatyRecord` (`negotiation.rs:349`), les afficher en rouge | 5 |
| 14 | Jauge : chance affichée vs issue déterministe | Le joueur voit « Accepterait (+N) » (`diplomacy_negotiation_tab.gd:117`) et une barre de chance ; l'issue est le seuil score ≥ 0 (ADR 0182, `ACCEPT_SCORE`) tandis que les propositions de l'IA au joueur utilisent `ai_min_chance` 60 (`diplomacy_eval.rs:49`). Cohérent, mais la barre suggère un aléa qui n'existe pas | 2 | S | Marquer la barre d'un repère à 50 % et libeller « seuil » ; ne pas ajouter d'aléa | — |

## 3. Top 10 par impact / coût

### 1. Prévision des alliés avant déclaration de guerre (#8) — S
- `answers_call_to_arms` (`sc/diplomacy/ai.rs:172`) est pure : l'appeler dans `war_verdict` (`godot-bridge/src/campaign_sim_diplomacy.rs:233`) pour chaque allié de la cible.
- Retourner dans `reasons` des lignes `« X viendra à son secours »`, `« X hésite »` (attitude entre −20 et 0), `« X refusera : trésorerie vide / épuisé »` et la ligne prestige (−20 sans motif, −30 trêve).
- Si le vassal-suzerain est concerné, appeler `feudal::answers_host` comme à `ai.rs:183`.
- Tests : `ai/tests` ou `sim-campaign/tests/diplomacy/` : cible avec 2 alliés (un sain, un en banqueroute), vérifier les libellés. UI : aucune modification (les `reasons` sont déjà affichées).

### 2. Étiquette d'attitude + durée des modificateurs (#6, #7) — S
- `sc/diplomacy/attitude.rs` : retourner aussi `turns_left` par modificateur (calcul `expires_turn - state.turn`, vide pour `FOREVER`).
- `data/rules/diplomacy.json` : bandes `attitude_bands` [{min, label}] + schéma `data/schemas/`.
- Pont `get_diplomacy` : champs `attitude_band`, `attitude_reasons[].turns_left`.
- UI : `diplomacy_head_section.gd:27` « Cordial (+27) » ; infobulle de liste (`diplomacy_faction_list.gd:80`) « (encore 12 tours) ».
- Test : `sim-campaign/tests/diplomacy/` : modificateur 40 tours → `turns_left` décroît à chaque tour.

### 3. Fiche d'une faction : ses alliances et ses guerres (#11) — S
- Étendre `DiplomacyEntry` (`sc/diplomacy/view.rs`) avec `allies`, `enemies`, `vassals` (noms).
- `diplomacy_head_section.gd` : une ligne « Alliés : … · Ennemis : … » après les faits ; les noms ouvrent la fiche (signal `faction_chosen`).
- Test : smoke UI existant (`smoke_ui`) + test de vue Rust sur l'ordre stable des noms.

### 4. Révocation et durée de l'accès militaire / commerce (#5) — S
- `orders/order.rs` : `RevokeMilitaryAccess { target }` ; `orders/mod.rs` l'applique via un nouveau `CampaignState::revoke_military_access` dans `diplomacy/ties.rs` (retire de `ledger.military_access`, `add_modifier(target, faction, -15, "Accès retiré", 20)`).
- Option : champ `turns: Option<u32>` sur `MilitaryAccess` et `TradeAgreement`, échéance traitée à côté de `negotiation.rs:1080`.
- UI : bouton « Retirer l'accès militaire » dans `diplomacy_actions_section.gd` si `entry.access_given` (`get_trespass`).
- Tests : accès accordé puis retiré → `passage` redevient violation (`passage.rs:87`) ; IA bloquée à nouveau par `ai/src/grid.rs`.

### 5. Ligue anti-hégémon (#9) — M
- `data/ai/diplomacy.json` : bloc `league { province_share: 0.18, power_ratio: 2.0, attitude: -25, join_bonus: 15, duration_turns: 40 }` + schéma.
- `sc/diplomacy/ai.rs` : `fn hegemon(state) -> Option<FactionId>` : une faction dont la puissance dépasse `power_ratio` × la 2e et qui tient plus de `province_share` des provinces (hors joueur par défaut).
- `attitude.rs` : ligne « Puissance menaçante pour tous » (−25) envers l'hégémon ; `plan_alliances` accepte des alliés d'hégémon rivaux ; `war_target` ajoute un bonus aux cibles hégémoniques ; événement « Ligue contre X » une fois par déclenchement.
- Liaison avec RX `equil` (annexion totale coûteuse) : pas de double comptage, la ligue est le coût social.
- Tests : état synthétique à 1 hégémon et 4 voisins → alliance mutuelle des voisins en ≤ N tours ; sonde `campaign_probe` : part des provinces du premier plafonnée.

### 6. Clause « Rejoindre la guerre contre X » (#1, #4) — M
- `negotiation.rs` : `Article::JoinWar { giver: Party, target: FactionId }`, clé `join_war` dans `key()`.
- `check.rs` : refus si cible = l'autre partie, déjà en guerre, allié du giver (`FEUDAL`/alliance), trêve (ou parjure -40 prévenu), giver en banqueroute.
- `value.rs` : base = `join_war.ratio` appliqué à la puissance de coalition, + `common_enemy`, − `weariness`, − `friend_of_target`, + attitude ; ajouter poids dans `treaty_weights.join_war`.
- `apply.rs` : `declare_war(giver, target)` avec casus belli « défense d'un allié » (déjà : `queries.rs:205`).
- `label.rs` / UI : entrée dans « Vous demandez » (cible choisie parmi les ennemis du joueur) ; `treaty_explain.rs` : raisons de refus.
- Tests : accepté si allié et ratio ; refusé si fatigue ; guerre déclarée + événement.

### 7. Demande de soutien d'un allié au joueur (#3) — M
- `negotiation.rs` : `Article::AllyCall { aggressor }` classé `is_imposed` (`:212`) ; `Offer` créée dans `call_to_arms` (`war.rs:146`) quand `ally == player_faction` au lieu de `true` (`ai.rs:192`).
- Accepter : `start_war(player, aggressor)` + événement ; refuser/expirer (`upkeep.rs:36`) : mêmes effets que le refus IA (`war.rs:176-200`, −30, alliance rompue).
- UI : l'offre apparaît déjà dans `diplomacy_offers_section.gd` ; texte « X est attaqué par Y et réclame votre aide ».
- Tests : joueur allié, attaque sur l'allié IA → offre ; accepter, refuser, expirer.

### 8. Pacte de non-agression et alliance défensive (#2) — M
- `Article::NonAggression { turns }` : `apply` ajoute à un nouveau `ledger.non_aggression: BTreeMap<FactionId, u32>` ; `declare_war` (`war.rs:12`) le lit comme la trêve (`truce_broken`, −40 de parjure).
- `Alliance { kind }` : serde par défaut `military` pour les sauvegardes ; `call_to_arms` n'appelle que les défensives/militaires pour une guerre défensive et `ally_war_to_join` seulement les militaires.
- Pesée : défensive plus facile à obtenir (`military_commitment` −15 → −5), pacte : base +10.
- Tests : déclaration contre un signataire = parjure ; migration de sauvegarde.

### 9. Ultimatums et tributs proposés par l'IA (#10) — M
- `ai/src/diplomacy_eval.rs` : `plan_ultimatum` (phase 5 de `PLAN_PERIOD`) : forte puissance, voisin, casus belli, cible plus faible ; propose `Treaty[CedeProvince | Tribute]` au joueur comme `Offer` expirant sous 2 tours.
- Expiration/refus : l'IA déclare la guerre au tour d'après (case `last_war_declared` réutilisable) ; ajouter un modificateur « Ultimatum refusé » côté IA pour que `war_target` la prenne.
- Tests : offre créée, guerre déclarée après refus, pas de répétition (cooldown `OFFER_COOLDOWN`).

### 10. Historique des ruptures et parjures (#13) — S
- `TreatyRecord` (`negotiation.rs:349`) : champ `kind: signed|refused|broken`.
- Écrire à `break_alliance` (`ties.rs:39`), `break_trade_agreement`, `declare_war` avec `truce_broken`, `call_to_arms` refusé.
- `diplomacy_history_tab.gd` : couleur et libellé « rompu ». Test : après `break_alliance`, `get_treaty_history` contient l'entrée.

## 4. Refontes lourdes (L)

### A. Confédération et union volontaire (#12)
- Aujourd'hui : l'héritage de titres peut absorber une faction (`feudal/inherit.rs:261`), la vassalité existe, mais deux factions du même sang ne peuvent s'unir par traité. Proposition : `Article::PersonalUnion` entre deux souverains parents (même maison ou mariage) : l'un devient titulaire du titre de l'autre (`feudal::conquer_title`), l'autre faction subsiste comme royaume sous la même couronne (cas Angleterre-Aquitaine, Castille-Léon, Pologne-Lituanie). Nécessite : décision ADR, transfert d'ordres de faction, sauvegarde, effets sur victoire.

### B. Diplomatie à tours multiples : pourparlers et crises
- TWW3 fait réagir l'IA immédiatement ; ici les offres IA vivent 2 tours (`OFFER_LIFETIME`, `mod.rs:42`). Un système de « crise » (incident de frontière → note de protestation → ultimatum → guerre) donnerait une chaîne lisible du casus belli à la guerre, nourrie par `passage.rs` (violations) et les prétentions. Coûteux : événements, UI de crise, rééquilibrage de la cadence de guerre RX.

### C. Opinion structurée par catégories
- Remplacer la liste de textes `(String, i32)` d'`attitude()` par des catégories typées (`OpinionCategory`) pour permettre filtres, plafonds par catégorie (déjà partiel : `opinion_caps`, `data/rules/diplomacy.json`) et traduction : aujourd'hui `opinion_motive` fait correspondre des chaînes françaises (`mod.rs:60`), fragile. À faire avec #6.

## Remarques de méthode
- Vérifiés dans le code : refus et acceptation (`negotiation.rs:471-500`, ADR 0182), seuil d'acceptation, perte de l'accès militaire à la guerre seulement (`negotiation.rs:1080`), cap `MAX_ALLIANCES = 4` (`ai.rs:18`), repos de guerre `WAR_REST_TURNS = 12` (`ai.rs:16`).
- Non vérifié : comportement en partie réelle des offres IA (fréquence réelle de `plan_treaties` vers le joueur) ; à mesurer avec `campaign_probe` (`ai/examples/`) avant d'ajouter des types d'offres.
