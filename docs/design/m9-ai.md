# M9 — IA de campagne et de bataille : spécification

Date : 2026-09-23. Objectif : remplacer l'IA minimale (M2) par une IA de campagne stratégique et donner
à la bataille temps réel (M7) une IA tactique. Design : `docs/design/2026-09-23-cent-ans-design.md` § 4.8.

## 1. IA de campagne (`core/crates/ai/src/campaign.rs`, `ai::plan_turn`)
Fonction pure `plan_turn(state, data, faction) -> Vec<Order>`, déterministe, utilisée par le pont
(`CampaignSim.end_turn` appelle `end_turn_with(ai::plan_turn)`) ; l'IA minimale reste celle des tests
de la simulation.
- **Évaluation stratégique** : ennemis actifs, puissance propre/coalition, menace par province amie
  (armées hostiles dans un rayon de 2 déplacements), fronts ; posture `offensive` (rapport ≥ 1,2 ou
  agressivité ≥ 60), `défensive` sinon.
- **Objectifs militaires** : chaque armée se voit attribuer au plus un objectif, par valeur décroissante :
  1. défendre une province amie menacée si l'armée peut l'atteindre et n'est pas écrasée (rapport ≥ 0,7) ;
  2. assiéger une province ennemie : valeur = revenu + 50 si capitale + 30 si prétention + 20 si déjà
     occupée par un allié à côté, divisée par (1 + distance), à condition que la puissance de l'armée dépasse
     1,5 × la défense ; poursuivre un siège en cours ;
  3. chevauchée (posture `raid`) en territoire ennemi riche si l'armée est trop faible pour assiéger et que
     la faction est agressive (Angleterre historiquement) ;
  4. regroupement : rejoindre la plus grosse armée amie à portée ; fusion des armées dans la même province ;
  5. retraite vers la capitale si l'armée est très affaiblie (effectif < 40 %) ou en territoire ennemi en
     hiver avec un ravitaillement < 30.
- **Économie** : budget d'entretien militaire ciblé (60 % du revenu en guerre, 30 % en paix), recrutement de
  la meilleure unité disponible (puissance/coût) dans les provinces qui peuvent recruter (capitale en
  priorité, puis provinces frontalières menacées), garnisons minimales dans les provinces frontalières (P1 : « frontière » = port ou voisine d'une
  province tenue par une autre faction, `CampaignState::is_frontier`, même classification que le setup 1337) ;
  construction du bâtiment de meilleur rendement (revenu, santé, ordre public si mécontentement élevé)
  quand le trésor dépasse une réserve de 2 tours d'entretien ; impôts : haut en guerre si mécontentement
  < 30, bas si mécontentement > 55 ; désarmement en cas de dette.
- **Personnages** : nommer des gouverneurs (meilleure gouvernance disponible) dans les provinces les plus
  riches, donner un général (meilleur commandement présent) à chaque armée sans chef, dépenser les points
  de compétence (branche selon le rôle), marier les membres adultes célibataires de la maison régnante
  (candidat de la même faction ou d'un allié).
- **Recherche et diplomatie** : réutilise `research::ai_choose_research` et `diplomacy::plan_diplomacy`.
- Tests (≥ 8) : une faction en guerre plus forte assiège, une province menacée est défendue, l'IA ne
  s'endette pas durablement, recrute dans le budget, nomme gouverneurs et généraux, déterminisme 20 tours,
  pas d'ordres invalides en boucle (taux de refus < 20 % sur 40 tours), 100 tours sans panique pour toutes
  les factions.

## 2. IA de bataille — fait (`core/crates/sim-battle/src/ai.rs`)
Plans par rôle d'unité pour `sim-battle` : ligne d'infanterie au centre, tireurs devant puis repli derrière
la ligne, cavalerie sur les ailes qui charge les flancs ou les tireurs isolés, réserve ; réactions : combler
une brèche, faire face à une attaque de flanc, retirer les unités en déroute, poursuivre un ennemi en
déroute avec la cavalerie ; posture défensive (pieux, colline) si le camp est plus faible (Crécy, Azincourt).
Commandes émises via l'API de commandes de `sim-battle`, toutes les 2 s simulées. Tests : l'IA bat une IA
passive à forces égales, la cavalerie charge un flanc exposé, les archers se replient au contact.

**Réalisation.** L'IA vit dans `sim-battle` (`ai::plan`, appelée par `BattleSim::step` tous les
`AI_PERIOD` = 2 s) et non dans le crate `ai` : la simulation l'appelle elle-même pour le camp que le joueur
ne commande pas, et `ai` dépend déjà de `sim-campaign` (qui dépend de `sim-battle`). Fonction pure de
l'état, déterministe, qui n'émet que des `Command` validées comme celles du joueur.
- Rôles : ligne d'infanterie (le fantassin le plus en arrière est la réserve à partir de 4 ; les tireurs
  à court de munitions rejoignent la ligne), tireurs, cavalerie, engins, régiment du général.
- Posture : défensive si le camp pèse moins de 0,85 fois l'ennemi (puissance = effectif × qualité ×
  moral ; un assaillant n'attend que 4 min, un défenseur 8 min) : meilleure hauteur près du déploiement,
  tireurs devant qui plantent leurs pieux, contre-charge à 45 m. Sinon offensive : les tireurs avancent
  à portée et tiennent un duel (tant qu'ils ont ≥ 20 % de munitions et ne le perdent pas, 8 min au plus),
  puis la ligne avance, chaque régiment engage l'ennemi en face (course à 60 m).
- Tireurs : repli derrière la ligne (course) dès qu'infanterie ou cavalerie ennemie est à 70 m ou au contact.
- Cavalerie : contre-charge de la cavalerie ennemie à 160 m, charge des tireurs isolés (aucune troupe de
  mêlée à 80 m, jamais de face sur des pieux), contournement puis charge de flanc/dos d'un régiment
  ennemi engagé (jamais un schiltron ou des piques), poursuite des fuyards à 350 m, charge d'un régiment
  ébranlé (moral < 40 ou moitié des effectifs) ; sinon tient l'aile ; le général reste derrière le centre.
- Réactions : la réserve comble une brèche (régiment de ligne en déroute ou vacillant) ou frappe un
  ennemi qui prend un ami de flanc ; un régiment pris de flanc se tourne vers l'assaillant ; un régiment
  ébranlé (moral < 28, moitié des effectifs) est retiré de la mêlée s'il existe une réserve.
- Sièges : voir `m8-sieges.md` § 2 (engins sur le pan de façade le plus faible, bélier sur la porte,
  tours sur la façade, infanterie qui attend hors de portée puis entre par la première ouverture,
  escalade depuis les tours ou avec des échelles ; garnison qui tient le rempart, repousse les grimpeurs
  et bouche les ouvertures).
- Rééquilibrage du moteur M7 (les batailles d'IA duraient ≈ 2 min et le défenseur gagnait presque
  toujours) : corps à corps 0,05 → 0,035, tir 0,4 → 0,3, perte de moral par pertes 120 → 60, contagion
  des déroutes 1 → 0,4 point/s par régiment en fuite. Mesure (`probe -- ai`, 10 graines, armées
  identiques) : 2 × 20 régiments ≈ 8 min, assaillant 4/10 ; 2 × 10 ≈ 7,5 min, 5/10.
- Tests (6, `sim-battle/tests/ai.rs`) : l'IA bat une IA passive dans les deux sens (3 graines), la
  cavalerie charge un flanc exposé, les archers se replient derrière leur ligne, un défenseur plus faible
  garde la hauteur et plante ses pieux, les batailles d'IA durent 5-15 min sans vainqueur systématique
  (8 graines), déterminisme.

## 3. Critères de fin
Tests verts ; sonde `cargo run --release -p ai --example ai_probe` sur 100 tours : guerres menées, sièges
réussis, pas de faction en faillite durable ; smoke vert ; `docs/status.md` à jour.

## 4. F4 — guerre de Cent Ans vivante (session 4)

Objectifs : `docs/design/v2-finalisation.md` § 2.2. Mesure : `cargo run --release -p ai --example
century_probe [tours] [graines…]` (5 graines × 464 tours par défaut, en parallèle ; `VERBOSE=1` imprime les
guerres et paix France-Angleterre, les banqueroutes et trésors dormants par faction). Dans la sonde, la France
(« joueur ») répond aux offres comme l'IA les jugerait (`evaluate`) ; sans cela aucune paix proposée à la France
n'était signée et l'Angleterre conquérait tout le royaume.

**Diplomatie** (`sim-campaign/src/diplomacy.rs`, section « Diplomatic AI ») :
- *Guerre de prétention* : une faction qui revendique le trône d'une autre ou des provinces qu'elle possède
  (`claim_stakes`) déclare la guerre dès la fin de la trêve si agressivité ≥ 45, attitude < 20 et rapport
  (sa coalition / la cible seule) ≥ 0,5 avec alliés ou tête de pont (provinces limitrophes), ≥ 0,8 sinon.
  Guerre opportuniste sans prétention : agressivité ≥ 60, casus belli, rapport de coalitions ≥ 1,5 (règle M5).
- *Tempo* (`war_ready`) : pas de régence, souverain libre, trésor positif ≥ 2 saisons d'entretien, revenu ≥
  entretien (ou trésor ≥ 8 saisons d'entretien) ; 12 tours entre deux déclarations ; pas de nouveau front si
  les ennemis qui pressent (voisins ou occupants) pèsent plus de la moitié de nos forces.
- *Cobelligérance* : un allié apprécié (attitude > 10) entre dans la guerre de son allié contre un ennemi
  voisin ou revendiqué si sa coalition pèse ≥ 0,6 fois celle de l'ennemi.
- *Alliances* contre nos rivaux (ennemis, cibles de prétentions, prétendants) : partenaire qui partage un
  rival, ou qui craint notre rival plus qu'il ne nous aime pas (contrepoids) ; au plus 4 alliances ;
  évaluation : « Rival commun » +15, « Contrepoids à un voisin menaçant » +10, « Allié de nos rivaux » −40.
- *Appel aux armes* (`answers_call_to_arms`) : vassal loyal, suzerain protecteur, sinon attitude > 0 ou
  rancune commune contre l'agresseur (attitude > −20) ; refus seulement si le trésor est vide.
- *Vassal opportuniste* (Bourgogne) : loyauté < 50 et suzerain battu (score ≤ −25) par une coalition plus
  forte → paix blanche avec le vainqueur puis déclaration d'indépendance.
- *Paix* : paix blanche si chacun l'accepte ; vainqueur (score > 40) : les 2 provinces occupées ; vaincu
  (score < −40) : cède toutes les provinces occupées (« Conquêtes reconnues » = score du vainqueur). Raisons
  nouvelles : « Guerre sur un autre front » +15, « Prétention au trône » −10, « Provinces revendiquées » −5.

**Armées** (`ai/src/campaign.rs`) : une prétention au trône rend revendiquées toutes les provinces de la couronne
(débarquements anglais en France) ; les deux dernières provinces libres d'un royaume sans prétention de
l'assaillant ne sont ni assiégées ni pillées (`LAST_BASTIONS`) : la paix décide (l'Écosse survit).

**Économie** : trésor au-delà de 3 saisons de revenu brut dépensé en 8 tours (armée, bâtiments), don annuel de
5 % de l'excédent à l'Église tant que la faveur < 70 ; impôts bas seulement si mécontentement > 55 ; budget
militaire = part (70 % en guerre, 40 % en paix) du revenu net laissé par les bâtiments ; un bâtiment n'est
lancé que si son entretien tient dans l'excédent ; licenciement anticipé si le trésor ne couvre pas 8 (guerre)
ou 3 (paix) saisons de déficit, ou dès qu'un déficit de paix n'est plus couvert par l'excédent ; dettes
remboursées en 8 tours. Règles (`economy.rs`) : cour opulente 20 % du trésor au-delà de 6 saisons (M10 : 3 %
au-delà de 8) ; bâtiments d'une province assiégée sans entretien, entretien réduit de la moitié de la dévastation.

**Mariages** : chaque printemps, le souverain puis l'héritier puis l'aîné célibataire de la maison ; conjoint à
±15 ans, épouse ≤ 35 ans, valeur = prestige + attitude + 40 (maison régnante étrangère) + 20 (allié), refus
exclus d'avance (`evaluate`), offre au joueur possible.

**Dynastie** : Pierre Ier de Portugal et Amédée VI de Savoie ajoutés (héritiers de 1337) ; un souverain vaincu
meurt au combat à 1 % (général : 5 %). Tests : `ai/tests/f4_war.rs`, `sim-campaign/tests/f4_succession.rs`.

## 5. G2 — alignement historique

Mesure : `century_probe` (qui imprime aussi les alliances pendant les guerres France-Angleterre, la part des
tours où l'Angleterre domine le royaume et les banqueroutes de l'Écosse). Code : `ai/src/support.rs`
(subsides), `ai/src/alignment.rs` (changements de camp), `ai/src/campaign.rs` (économie). Tests :
`ai/tests/g2.rs`. `sim-campaign` n'est pas modifié : l'ordre `SendGift` existant sert aux subsides.

- *Subsides* : chaque tour, avant l'économie, un allié au moins 3 fois plus riche (revenu brut) qui partage
  un rival (`rivals`) paie à un allié endetté de quoi couvrir sa dette et 2 saisons de déficit, pris sur le
  tiers de son trésor au-delà d'une saison d'entretien (l'or français de l'Auld Alliance).
- *Pas de monnaie affaiblie* pour un royaume dont les bâtiments mangent la moitié du revenu : l'inflation de
  leur entretien dépasse le seigneuriage (spirale de l'Écosse réduite à Fife).
- *Tirage par campagne* (`campaign_roll`, splitmix64 de la graine, sans toucher au RNG) : l'histoire hésite,
  environ une campagne sur deux (`HISTORY_PERMILLE` = 500) voit chaque bascule.
- *Révolte de la laine* : un vassal (loyauté < 60) frappé par l'embargo d'un ennemi de son suzerain
  (Flandre, embargo anglais de 1337) déclare son indépendance et propose l'alliance à l'embargo ; celui-ci
  lève l'embargo sur un royaume qui combat ses ennemis. L'événement `evt_artevelde` reste en place.
- *Alliances dynastiques* : un royaume en guerre propose l'alliance à un prince indépendant voisin de son
  ennemi qui le préfère (attitude ≥ 20 et > attitude envers l'ennemi + 10 ; marge 0 pour un parent de nos
  alliés — attitude ≥ 20 envers l'un d'eux), sans plafond d'alliances (Hainaut, Brabant, Gueldre).
- *Défection bourguignonne (Troyes)* : un vassal (loyauté < 90) ou un allié change de camp quand
  l'envahisseur domine le royaume de son suzerain — capitale de 1337 ou ≥ 8 provinces du royaume (régions
  où la couronne possédait ≥ 3 provinces en 1337, Guyenne et Ponthieu compris) — et que la couronne a perdu
  un quart de ses terres ou un score ≤ −25 : paix blanche avec l'envahisseur, guerre d'indépendance (ou
  rupture d'alliance), offre d'alliance. Un tirage par décennie ; jamais vers une couronne qui revendique nos
  terres (l'Écosse ne passe pas à l'Angleterre).
- *Trésors dormants* : le trésor au-delà de 3 saisons de revenu est dépensé en 4 tours (au lieu de 8) et
  achète des recrues à son rythme (1 recrue par 1 500 livres dépensées par tour, 16 au plus).
