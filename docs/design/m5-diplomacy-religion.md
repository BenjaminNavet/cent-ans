# M5 — Diplomatie et religion : spécification

Date : 2026-09-23. Objectif : toutes les mécaniques diplomatiques du design (§ 4.4) et la religion (§ 4.5) :
guerre et paix négociées, alliances et appel aux armes, trêves datées, embargos, vassalité avec loyauté et
rébellion, mariages entre factions et prétentions (casus belli), attitude des IA calculée ; Papauté
d'Avignon, faveur pontificale, excommunication, médiation, Grand Schisme (1378-1417), hérésies (Lollards,
Hussites).

## 1. Données
- `data/factions/*.json` : champ optionnel `claims` : `[{kind: "throne"|"province", faction?, province?, note}]`.
  1337 : Angleterre → trône de France (Édouard III par sa mère Isabelle), provinces de Guyenne confisquées
  (`prov_guyenne`/`prov_gascogne` selon les ids) ; France → Guyenne et Ponthieu ; Écosse ↔ Angleterre
  (Balliol/Bruce), Castille ↔ Portugal. Schéma `faction.schema.json` et `data-model` mis à jour.
- `data/religions` : `rel_catholic` (Avignon, unifiée avant 1378), `rel_catholic_rome` (obédience romaine
  pendant le Schisme), `rel_lollard` (Angleterre, dès 1380), `rel_hussite` (Bohême/Empire, dès 1415) :
  ajouter aux religions `schism: {from, to}`, `heresy: {origin_provinces[], appears}` si utile (champs
  optionnels, schéma à jour).

## 2. Simulation (`sim-campaign`, nouveaux `diplomacy.rs`, `religion.rs`)
### 2.1 État
- `FactionState` gagne : `embargoes: BTreeSet<FactionId>` (embargos imposés), `suzerain: Option<FactionId>`,
  `loyalty: u8` (0-100, vassaux), `claims: Vec<Claim>`, `modifiers: Vec<OpinionModifier{with, value,
  reason_fr, expires_turn}>`, `war_scores: BTreeMap<FactionId, i32>` (-100..100, point de vue de la faction),
  `religion: ReligionId`, `papal_favor: u8`, `excommunicated_until: Option<u32>`, `offers: Vec<Offer>`
  (propositions reçues par le joueur). `ProvinceState` gagne `heresy: u8` et `heresy_religion`.
  `CampaignState` gagne `schism: bool`. `state_version` +1.
### 2.2 Attitude
`attitude(a, b) -> (i32, Vec<(String, i32)>)` bornée -100..100, avec raisons en français : personnalité
(`ai_personality.diplomacy` - 50) / 2 ; guerre -50 ; trêve -10 ; alliance +30 ; lien matrimonial entre
maisons régnantes +15 ; même maison +20 ; même religion/obédience +10, obédience différente -20, hérésie
-40 ; excommunié -30 ; ennemi commun +20 ; menace (voisin dont la force militaire > 1,5 × la sienne) -15 ;
prétention de b sur a -25 ; embargo -20 ; vassal envers suzerain : (loyauté - 50) / 2 ; modificateurs
historiques (trahison, guerre sans casus belli, mariage, cadeau, médiation) avec expiration.
### 2.3 Ordres et acceptation
Ordres (joueur et IA) : `declare_war {target}`, `propose_peace {target, cede[], pay}` (cession de
provinces occupées, tribut en livres ; paix blanche si vides), `propose_alliance {target}`,
`break_alliance {target}`, `set_embargo {target, active}`, `demand_vassalage {target}`,
`release_vassal {target}`, `send_gift {target, amount}`, `answer_offer {offer, accept}`,
`request_papal_mediation {target}`, `donate_to_church {amount}`, `choose_obedience {religion}`.
- `evaluate(proposal) -> Evaluation {accept: bool, score, reasons[(fr, valeur)]}` pur : utilisé par l'IA
  et affiché au joueur avant envoi. Paix : score de guerre, lassitude (tours de guerre, trésor, provinces
  ravagées), exigences ; alliance : attitude, ennemi commun, force ; vassalité : rapport de forces ≥ 3 et
  attitude.
- Guerre sans casus belli (prétention, embargo, rupture de trêve = pire) : prestige -20, -20 d'attitude
  chez toutes les factions (10 ans), rupture de trêve : -40 et excommunication possible.
- Paix : trêve de 5 ans (20 tours), provinces cédées changent de propriétaire (garnisons retirées),
  tribut payé immédiatement, modificateur.
- Appel aux armes : un allié attaqué appelle ses alliés ; chacun rejoint si attitude > 0, sinon l'alliance
  est rompue (événement, modificateur -30). Les vassaux suivent toujours le suzerain sauf si loyauté < 30.
- Score de guerre : batailles gagnées (+5/+10), provinces ennemies occupées (+ valeur), provinces perdues,
  mis à jour dans les hooks de bataille et de siège ; affiché.
- Vassalité : tribut 10 % du revenu du vassal au suzerain ; loyauté : +attitude, - tribut, - guerres
  impopulaires, + suzerain fort ; loyauté < 20 : rébellion (le vassal déclare l'indépendance, guerre avec
  le suzerain, événement). Flandre 1337 : loyauté basse (embargo anglais sur la laine).
- Embargo : revenu commercial de la cible -15 %, de l'imposeur -5 % ; mécontentement des bourgeois de la
  cible +5.
- Mariages interfactions (moteur M4) : un mariage entre membres des maisons régnantes crée un lien
  (`MarriageTie`, +15) ; un enfant d'une mère issue d'une maison régnante étrangère donne à sa faction une
  prétention au trône de cette faction si la lignée s'éteint (`no_heir`) : la faction prétendante reçoit un
  casus belli ; si elle est en paix avec la faction éteinte et que l'attitude > 50, union personnelle
  (la faction éteinte devient vassale).
- Propositions IA → joueur : l'IA propose paix, alliance, mariage au joueur (au plus une offre par
  faction et par an) ; elles vont dans `offers` (expirent après 2 tours). IA ↔ IA : réglé immédiatement.
- IA diplomatique minimale : déclare la guerre avec casus belli si rapport de forces ≥ 1,5 et agressivité
  élevée, propose la paix quand le score de guerre est bas ou la lassitude haute, recherche des alliances
  contre l'ennemi principal, lève les embargos inutiles.
### 2.4 Religion
- Faveur pontificale 0-100 : +piété du dirigeant, +bâtiments religieux, +dons (`donate_to_church`),
  -guerre contre un allié du pape, -hérésie tolérée. Excommunication (faveur < 10 et agression contre un
  catholique, ou rupture de trêve jurée) : 10 ans, -30 d'attitude catholique, mécontentement +10,
  les vassaux perdent 20 de loyauté ; levée si faveur > 40.
- Médiation pontificale (`request_papal_mediation`) : coût 1 000 livres, faveur ≥ 30 ; la cible reçoit
  une proposition de trêve de 2 ans avec +30 d'acceptation (historique : Benoît XII et Clément VI).
- Grand Schisme : à l'automne 1378, `schism = true` ; les factions choisissent leur obédience :
  historiques par défaut (Avignon : France, Écosse, Castille, Aragon, Navarre, Savoie, Naples/Anjou ;
  Rome : Angleterre, Empire, Flandre, Portugal, Milan, Gênes) ; le joueur reçoit une offre
  `choose_obedience` (4 tours). Obédience différente -20. Fin en 1417 (concile de Constance) : tous
  reviennent à l'obédience romaine, événement.
- Hérésies : Lollards dans les provinces anglaises à partir de 1380, Hussites en Bohême/Empire à partir
  de 1415 ; `heresy` croît avec le mécontentement et la faible satisfaction du clergé, décroît avec les
  bâtiments religieux et la piété du gouverneur/dirigeant ; hérésie > 60 : mécontentement +10, risque de
  révolte hérétique (révolte M3 avec texte dédié).
- Piété des personnages (M4) : effet `Piety` des traits ; un dirigeant pieux (> 70) +5 de faveur/an.
### 2.5 Tests (≥ 16)
Attitude (raisons présentes), déclaration avec/sans casus belli, paix acceptée/refusée selon le score,
cession de provinces, trêve respectée (mouvement hostile refusé), appel aux armes (rejoint/rupture),
embargo sur le revenu, tribut et loyauté vassale, rébellion vassale, prétention par mariage, offre IA
au joueur, médiation, excommunication, Schisme 1378 (obédiences historiques), fin 1417, hérésie qui croît,
sauvegarde round-trip, déterminisme 40 tours.

## 3. API GDExtension (`CampaignSim`)
- `get_diplomacy(faction) -> [{id, name, color, status: "war"|"truce"|"peace"|"alliance"|"vassal"|"suzerain",
  attitude, attitude_reasons[{text, value}], truce_turns_left, embargo_by_us, embargo_on_us, war_score,
  claims[{kind, text}], religion, religion_name}]` (toutes les factions vivantes sauf elle-même).
- `evaluate_proposal(dict) -> {accept, score, reasons[{text, value}]}` (même format que `submit_order`).
- `get_offers() -> [{id, from, from_name, kind, text, expires_in}]`.
- `get_religion_state(faction) -> {religion, religion_name, papal_favor, excommunicated, turns_left,
  schism, obedience_choice_pending}`.
- `get_province_state` gagne `heresy`, `heresy_religion`.

## 4. Interface Godot
- Panneau « Diplomatie » (bouton, touche P — D est prise par la caméra) : liste des factions (couleur, nom, statut, barre d'attitude,
  icônes trêve/embargo/prétention), fiche de faction sélectionnée : raisons de l'attitude, score de
  guerre, actions (déclarer la guerre, proposer la paix avec choix des provinces à céder et du tribut,
  alliance, embargo, vassalité, cadeau, médiation), prédiction « Accepterait / Refuserait » avec raisons
  avant envoi.
- Fenêtre d'offre reçue (accepter/refuser) en début de tour ; choix d'obédience au Schisme.
- Mode carte « Diplomatie » (touche N) : provinces teintées selon la relation avec le joueur (guerre
  rouge, allié bleu, vassal violet, trêve jaune, neutre gris) ; mode « Religion » (touche R, hérésie). Revenu : embargo -8 % par embargo subi, -3 % par embargo imposé (plancher 50 %) sur le revenu total..
- Panneau faction : section religion (faveur pontificale, excommunication, obédience).
- Journal : couleurs pour guerre/paix/alliance/rébellion/excommunication/schisme.
- Smoke : proposer la paix à l'Angleterre (évaluation lue), déclarer la guerre à une faction avec
  prétention, embargo, 20 tours sans erreur, offres lues.
- Captures : `docs/img/godot-diplomacy.png`, `docs/img/godot-diplomacy-map.png`.

## 5. Critères de fin
Tests verts, smoke vert, captures relues, 100 tours France sans crash avec au moins une paix et une
alliance changées, Schisme observé en 1378 dans une sonde (`cargo run -p sim-campaign --example
diplomacy_probe`), docs à jour.
