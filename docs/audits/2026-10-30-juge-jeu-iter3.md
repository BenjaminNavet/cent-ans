# Juge de jeu, itération 3 (boucle juge 2) : campagne et expérience du joueur (30/10/2026)

Base : `fusion-a` (c3db29c39 : CAMP-R6 8 maisons, ECO-R1c, XP-R3a). Regard neuf, sans lecture des rapports de juge précédents.
**Économie et politique fiscale : revue suspendue par le joueur** ; cette note ne juge que la campagne et l'expérience du joueur.
Les chiffres de trésor et d'armée ne servent qu'à lire la boucle de décision, aucun lot économique n'est proposé.

## 0. Méthode et limites

- Sonde `eco_series` (IA aux commandes de France, Angleterre, Bourgogne, Bretagne ; 3 graines ; 120 tours = 30 ans) : trajectoires de
  provinces, hommes, prestige, trésor (sortie : `out/eco.jsonl`, non versionnée).
- Pilote de juge `core/crates/seed-bots/examples/judge_play.rs` (exemple jetable, non commité, dev-dependance `game-api`) : l'IA joue
  la maison du joueur ; à chaque saison on imprime ce que le joueur affronterait (Audience : Affaires, À surveiller, Nouvelles ;
  événements de sa maison ; trésor, hommes, provinces, score de règne). 10 saisons (printemps 1337 à été 1339) pour France (graine 1),
  Navarre (1), puis Angleterre, Bourgogne, Bretagne, Écosse, Flandre, Castille (graine 2).
- Limite : une IA ne joue pas comme un humain (elle ne choisit jamais d'ambition, ne tranche pas les décisions de chronique, qui
  expirent « délai écoulé »). Les constats sur l'inertie du joueur-IA sont donc marqués comme tels ; ceux sur ce qui est offert
  (Affaires, missions, ressources disponibles) sont fiables.
- Pas de capture ni de client lancé (consigne : dumps ; le client est la couche de présentation, tous les constats portent sur le cœur
  et `game-api`). Seed bots `--quick` lancé en fin de séance ; résultat en § 5.

## 1. Synthèse

Le premier tour est riche (Audience, ambitions, missions, traités). Mais **la boucle des 10 premières saisons se vide très vite** :
selon la maison, soit le joueur ne peut rien faire (Navarre, Bretagne, Flandre : 1 province, 350 à 4 800 ₶ de revenu, 0 à 100 hommes),
soit il regarde une machine qui tourne seule (France : trésor 16 k à 100 k en 5 saisons, une seule armée de 620 hommes sans ordre
pendant 4 saisons, première bataille à la saison 8). Le score de règne reste à **0-4** après 10 saisons pour 6 maisons sur 8.
Les « choix lourds » annoncés par CAMP-R6 existent comme textes mais pas comme **tension** : rien ne se perd si on ne décide pas.

## 2. Constats

### Majeurs

**M1. Trois des huit maisons « jouables » n'ont pas de jeu (Navarre, Bretagne, Flandre ; l'Écosse et la Bourgogne à peine).**
Preuve (`judge_play`, 10 saisons) : Navarre 1 province, revenu 369 → 342 ₶/saison **en baisse**, trésor 369 → 1 024 ₶, **0 homme sous
les armes dès l'été 1337** (l'armée de départ de 100 hommes disparaît de la liste des armées), score de règne 0 aux 10 saisons, les
missions « Lever des troupes » (350 ₶) échouent par échéance (« Mission échouée, prestige −2 » aux saisons 4 et 9). Le conseiller
ne propose que « Un règne qui attend » (conseil `advice`) et « 1 localité sans chantier ». Bretagne : 1 province, 2 600 ₶, 240 → 400
hommes, reign 0. Flandre : 1 province, 4 800 ₶, reign 4. Écosse : 2 provinces, 800 ₶ de revenu pour 900 d'entretien (déficit
structurel, `treasury_swing` à 8 saisons sur 10), 100 hommes, régence. Bourgogne : 2 provinces, 100 hommes à la saison 9.
Ces maisons sont historiquement des acteurs de second rang, mais le jeu leur donne un levier unique (les 350 ₶ d'une mission) là où
l'histoire leur donnait des **voies politiques** (Navarre : Philippe d'Évreux, prétention champenoise et normande ; Bretagne : la
succession de 1341 est le sujet du jeu ; Flandre : Artevelde, l'embargo anglais, trois villes qui votent). Rien de cela n'est un
système jouable : ce sont des événements de chronique que l'IA laisse expirer. Recentrer sur 8 maisons n'a de sens que si chaque
maison a un « verbe » (ce qu'on y fait mieux que les autres).

**M2. Le départ de la France est une démonstration, pas une décision.** Preuve (graine 1) : trésor 16 000 → 47 184 (T0 fin) → 98 611 ₶ à la
saison 5, soit 15 saisons de revenu thésaurisées, avec 32 puis 28 localités sans chantier (l'avis `free_slot` le répète à chaque saison
sans rien chiffrer d'utile) ; 1 armée de 620 hommes, « Armée sans ordre : Philippe VI de Valois » aux saisons 0 à 4 ; entretien
d'armée 4 830 pour 36 000 de revenu ; première bataille de campagne à la saison 8 (Béarn, pertes 38 contre 55 : une escarmouche) ; le
siège d'Abbeville se rend « sans combattre » à la saison 5. À 30 ans de jeu (graines 1 et 2) la France IA tombe de 10 à 5 provinces
pendant que l'Angleterre en gagne 4 à 5 : la trajectoire que le joueur doit renverser n'est jamais lisible avant qu'il soit trop tard.
Un joueur humain a ici une surabondance d'argent et une pénurie de choix qui coûtent : la richesse ne se convertit pas en enjeu (pas de
levée à plafonner, pas d'allié à acheter, pas de dette d'honneur).

**M3. Le score de règne ne dit rien pendant les dix premières saisons.** `reign` : France 0, 3, 3, 3, 3, 3, 3, 3, 3, 4 (palier unique à
la saison 1) ; Navarre, Bourgogne, Bretagne, Castille 0 pendant 10 saisons ; Écosse 18 à la saison 9 (alors qu'elle perd de l'argent) ;
Angleterre 7. Le résultat est faiblement lié aux actes : la renommée « accumulée » (CAMP-R1) ne répond pas dans l'horizon où un joueur
décide d'un plan. Pas de jalon, pas de « tu es en retard sur ton acte ». Les ambitions (2 à choisir) restent une Affaire poussée
**à chaque saison** (10 sur 10 pour les 8 maisons) tant que le joueur n'a pas choisi : le bruit de l'Audience est donc dominé par une
Affaire qui se répète, quand le joueur ignore la consigne (un joueur-IA, mais aussi un joueur pressé).

**M4. Les décisions de chronique n'ont pas de prix au retard.** Les 7 décisions Angleterre, 4 Flandre (« Les prêteurs lombards » 4 saisons de
suite, « Artevelde, capitaine de Gand », « Le raid de Cadzand ») sont marquées bloquantes (`*`), puis résolues par « délai écoulé » :
`Chronique : La disette : Laisser faire le marché (délai écoulé)`. L'option par défaut est toujours « ne rien faire », neutre. Une
décision lourde doit avoir un défaut qui coûte, sinon elle est un menu optionnel. Les missions de même : « Lever des troupes » échoue
pour −2 de prestige, un prix que tout joueur paie sans regret.

**M5. La guerre est un fond sonore.** Quatre à huit « Armée ennemie près de X (200 hommes) » par 10 saisons pour Angleterre, Castille,
Écosse, Bourgogne : ce sont des détachements de 200 hommes annoncés comme des Affaires poussées (même rang qu'un siège ou qu'une
décision), sans enjeu (pas de province menacée nommée, pas de pays qui souffre, pas de date). Les offres de mission « Prendre
Yorkshire » données à l'Écosse (100 hommes, 800 ₶ de revenu) montrent que le générateur de missions ne connaît pas la puissance de la
maison. La guerre de 1337-1340 (Cadzand, la chevauchée, Tournai, Sluys, Buironfosse, « la drôle de guerre ») est réduite à des tours
d'escarmouche ; CAMP-R2 (guerre en saisons) n'a pas été livré.

### Moyens

**Mo1. Deux canaux de rappel parlent de la même chose.** À chaque saison : `free_slot` (« N localités sans chantier : le trésor permet d'y bâtir »,
France 32, 30, 28 localités) et `advice` (« Des chantiers vous attendent », « Une épargne qui dort », « Un règne qui attend »),
`idle_army` (« Armée sans ordre » 5 saisons de suite pour une armée qui attend légitimement un siège). Ils se répètent sans
escalade ni silence appris : un joueur apprend à les ignorer. Il faut un état « j'ai vu, plus de rappel pendant N saisons ».

**Mo2. Le journal de saison reste un catalogue.** 271 Nouvelles à la saison 1 de la France (22 lettres), 50 à 115 ensuite ; 8 à 21 événements
« de sa maison » par saison, dont 4 à 7 `BuildingCompleted` (« Péage achevé à X » ×5 en une saison), `Agent` (échec d'espion), `Income`.
Le résumé existe (Récit de la saison), mais la matière brute est une liste, pas une histoire : aucun fil causal visible entre « Péage achevé »
et le but de la maison. Les dix premières saisons n'ont pas de scène identifiable (« ce printemps, le siège d'Abbeville »).

**Mo3. L'IA (joueur-proxy) gaspille les départs.** France IA fait 3 sièges de la même place (Abbeville) en trois saisons (le siège redémarre :
`SiegeStarted` ×3), Navarre ne recrute pas, l'Écosse accepte une « alliance par le sang » et rompt avec la France aussitôt
(« refuse de soutenir France : l'alliance est rompue », saison 4). Ce n'est pas le joueur, mais c'est aussi l'adversaire du joueur :
l'IA qui ne mène pas de campagne coordonnée (CAMP-R5 non fait) ne donne pas de tension de campagne à la France de l'humain.

**Mo4. Les trajectoires séculaires font passer le joueur de maison puissante à spectateur.** Sonde 120 tours : France 10 → 5 provinces (graines 1 et 2),
Bourgogne 2 → 2 provinces avec prestige 4 à 61, Bretagne 1 → 2 ou 3. Les petites maisons n'ont pas d'expansion de règle (annexion,
dot, héritage) qui soit un moyen d'agir plutôt qu'un événement.

### Mineurs

- `treasury_swing` pour Écosse : « Le trésor baisse de 71 ₶ par saison : impôts et commerce (−12) » : la somme des raisons (−12) ne couvre pas −71,
  la ligne « autres éléments » manque quand les postes sont petits (lisibilité du Pourquoi).
- Navarre : l'avis « Le trésor baisse de 25 ₶ par saison : . » (T0) se termine par une raison vide (« : . »).
- Mission « Une alliance par le sang » proposée à chaque saison à Navarre, sans partenaire nommé dans l'extrait d'Audience.
- Les événements de saison de la France mêlent `Assembly` « États de langue d'oïl » et « États de langue d'oc » deux saisons de suite avec réponse
  « décline », sans que l'Affaire dise ce que le refus coûte (le Pourquoi existe côté causes, absent de l'Affaire).

## 3. Ce qui marche (à garder)

- L'Audience plafonne le bruit (3 Affaires, 3 à 5 lignes À surveiller) : les 8 maisons restent à 10 saisons sous le plafond.
- Les textes des chroniques sont spécifiques et datés (Artevelde, Black Agnes à Dunbar, le raid de Cadzand, les prêteurs lombards).
- Les 8 maisons ont des départs différents en argent et en hommes (12 000 à 875 ₶), la diversité des échelles est réelle.
- Le Récit de saison, la cloche et les causes sont en place ; la matière narrative est là, elle manque d'enjeux.

## 4. Diagnostic de fond

La simulation produit des **événements** (320 par tour au tour 0, 80-115 ensuite) mais pas des **situations** : un état durable, daté, que le joueur
peut changer et qui change contre lui s'il tarde (siège de Calais de 1346-47, chevauchée de 1346, Poitiers 1356, traité de Brétigny 1360).
Trois raisons structurelles :

1. **Les causes rares ne sont pas des objets de jeu.** Succession de Bretagne, embargo flamand, prétention anglaise, Navarre-Évreux, Écosse-régence :
   ce sont des événements de chronique (une carte, une option, une fin). Il faut des **dossiers** (états persistants avec horloge).
2. **Le joueur n'a pas de « tempo ».** Pas d'horloge de levée, pas d'hiver sans campagne, pas d'échéance de trêve annoncée : l'arrêt du jeu est le
   même toute l'année. CAMP-R2 (saisons de guerre) manque et il explique M2 et M5.
3. **Les maisons sont tarifées pareil.** Chaque maison utilise les mêmes leviers (mission, construction, recrutement, traités) ; il n'y a pas de
   verbe propre à la Flandre (villes), à l'Écosse (régence, alliance auld), à la Bretagne (succession).

## 5. Seed bots `--quick`

Lancé en fin de séance via `tools/heavy.sh`, dans l'attente d'un créneau (machine occupée par d'autres sessions) : voir la note de fin de
rapport dans la réponse de l'agent. Aucune conclusion de ce rapport n'en dépend.

## 6. Lots proposés (campagne et expérience ; économie suspendue)

Numéros d'ADR à réserver au lancement (prochain libre : 0824 ou suivant). Dépendances : **ECO-R2b (retenue)** est suspendu avec l'économie ;
**XP-R3b (livres)** touche l'interface des livres : les lots ci-dessous qui ajoutent un panneau passent par les livres existants
(Royaume, Chronique, XP-R3a) sans créer de fenêtre.

### Lot A. CAMP-V : un « verbe » par maison (Opus, conception puis Sonnet pour les données)
Spec : pour chacune des 8 maisons, un système court propre (4 à 6 règles, données dans `data/rules/house_verbs.json` + schéma) :
Flandre (votes des trois villes, embargo ouvert/fermé), Bretagne (succession 1341 comme dossier avec deux prétendants et soutiens),
Navarre (prétentions en Normandie/Champagne comme claims à faire valoir par mariage et rachat), Écosse (alliance auld, régence, raids frontaliers),
Bourgogne (Flandre et Artois par mariage), Castille (flotte et Reconquista), France (arrière-ban), Angleterre (parlement de guerre ; laine).
Critères : pour chacune des 8 maisons, `judge_play` 10 saisons : au moins 2 actions « verbe » disponibles par saison, aucune saison sans
action utile (`affairs + watch` utile >= 1) ; reign de la saison 10 >= 8 pour au moins 6 maisons sur 8 sans ordre d'IA ; test de sonde
`house_verbs.rs` (chaque verbe change une grandeur de la sonde de plus de 10 % sur 40 tours).
Fichiers : `core/crates/sim-campaign/src/` (nouveau module `house_verbs.rs`), `data/rules/house_verbs.json`, `data/schemas/`, `core/crates/game-api` (vue).
Dépend de : rien ; touche les mêmes ambitions que CAMP-R6b (relire `acts.json`).

### Lot B. CAMP-S : dossiers de crise avec horloge et défaut qui coûte (Opus)
Spec : remplacer les décisions de chronique-ponctuelles par des **dossiers** persistants (`Dossier { id, étape, horloge, soutiens }`) pour les
causes rares (succession de Bretagne, Artevelde, Cadzand, embargo, Black Agnes, légat). Chaque décision a un défaut daté qui coûte réellement
(prestige, loyauté d'un vassal, perte d'un allié, droit de claim) ; plus de « délai écoulé : laisser faire » neutre. La mission échouée coûte
plus que −2 prestige quand elle a été acceptée sous l'engagement d'un allié.
Critères : sur 3 graines et 10 saisons, au moins 2 dossiers ouverts par maison, 0 décision bloquante résolue « neutre » ; le coût du défaut
> coût du meilleur choix pour 80 % des décisions (test de données) ; Audience <= 3 Affaires conservé.
Fichiers : `core/crates/sim-campaign/src/chronicle*.rs`, `data/events/`, `data/rules/dossiers.json`, `game-api/src/agenda.rs`.
Dépend de : XP-R3b pour l'emplacement (livre Chronique) ; indépendant d'ECO-R2b.

### Lot C. CAMP-R2 (relance) : la guerre en saisons (Opus, le plus risqué)
Spec : levées d'été / quartiers d'hiver, trêve distincte de la paix, chevauchée qui pèse (dévastation visible, prise de butin), une **première
campagne datée** (1337 Cadzand, 1339 chevauchée de Thiérache, 1340 Tournai/Sluys) comme scénario d'IA : l'Angleterre IA conduit une
campagne avec but d'acte. Les détachements de moins de 300 hommes ne produisent plus d'Affaire (seulement une ligne À surveiller groupée).
Critères : première bataille d'envergure (>= 2 000 hommes par camp) avant l'automne de la 4e année dans 3 graines sur 3 ; pas d'Affaire
`enemy_army` pour < 300 hommes ; France IA : provinces à 30 ans >= 8 dans au moins 2 graines sur 3.
Fichiers : `core/crates/sim-campaign/src/` (mouvement, trêves, levées), `ai/src/campaign.rs`, `data/rules/`, `docs/audits/2026-10-29-campagne.md` § R2.
Dépend de : IA-PLAN-2b ; sérialiser avant tout lot IA de campagne.

### Lot D. XP-SOBRE : un rappel appris, un jalon de règne (Sonnet)
Spec : (1) les rappels `free_slot`, `advice`, `idle_army`, `ambition_choice` ne se répètent plus à l'identique : après 2 affichages sans action ils
passent en « ignoré 4 saisons » avec bouton « Rappeler ». (2) `ambition_choice` cesse d'être une Affaire poussée à chaque saison : poussée
saisons 0-1 puis 1 fois par an. (3) Un jalon de règne à la saison 4, 8, 12 : « en avance / à l'heure / en retard sur l'acte » avec ce qui manque
(`game-api::progress`). (4) Corriger les textes vides (« : . ») et la somme des raisons du `treasury_swing` (poste « autres éléments » toujours imprimé).
Critères : sur 10 saisons des 8 maisons, nombre d'Affaires + lignes de rappel répétées identiques <= 3 par type ; 100 % des raisons de `treasury_swing`
somment au delta ; aucun texte avec « : . » (test sur `audience.fr.json` rendu).
Fichiers : `core/crates/game-api/src/agenda.rs`, `progress.rs`, `data/rules/agenda.json`, `data/ui/audience.fr.json`, test dans `game-api/tests/`.
Dépend de : XP-R3b (aucun conflit si limité à `agenda.rs` et données) ; indépendant d'ECO-R2b.

### Lot E. CAMP-GEN : générateur de missions et de départs proportionnés (Sonnet)
Spec : les missions tiennent compte de la puissance de la maison (hommes, revenu, distance) : plus de « Prendre Yorkshire » pour 100 hommes ; chaque
maison reçoit au départ un petit « levier » lisible (Navarre 3 recrues offertes par mission de levée de plus de 350 ₶, Bretagne une milice permanente,
Flandre les milices des trois villes) pour ne pas être à 0 homme en 1337. Vérifier qu'aucune maison ne perd son armée de campagne avant la
saison 4 sans ordre (cas Navarre).
Critères : `judge_play` 10 saisons, 8 maisons : hommes sous les armes >= 100 à chaque saison ; 0 mission avec ratio garnison ennemie / hommes de la maison > 3 ;
`tests/missions_scaled.rs`.
Fichiers : `data/missions.json`, `core/crates/sim-campaign/src/missions.rs`, `data/rules/starting_armies.json`.
Dépend de : rien.

### Lot F (optionnel). PILOTE-JUGE : outillage de juge permanent (Sonnet)
Promouvoir `judge_play` (ce pilote) en exemple versionné `core/crates/seed-bots/examples/judge_play.rs` avec sortie JSON par maison, et une métrique de
« saison vide » (ni Affaire utile, ni dépense possible, ni action de verbe) : critère d'oracle des seed bots, borne <= 1 saison vide sur 10 par maison.
Dépend de : A et E pour définir « action utile ».

## 7. Ordre recommandé

D (rapide, parallèle) → E → A (en conception pendant que E se fusionne) → B (après XP-R3b) → C (après IA-PLAN-2b, seul sur l'IA de campagne) → F.
