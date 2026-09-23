# Lot F1 « Règles inertes » — état

Branche : `worktree-agent-ae66d663ee24e14ab`. Tests : `core/crates/sim-campaign/tests/f1_effects.rs`.

## Points
1. [x] Bâtiments : `Garrison`, `RecruitCost`, `Supply`, ciblage par classe (et par catégorie d'unité).
2. [x] Technologies : `army_upkeep`, `army_experience`, `recruit_cost`, `movement`, `production`,
   `siege_resistance`, `fortification_level`, `wealth`, `prestige` ; surplus de recherche conservé.
3. [x] Traits/compétences : `research_civil`/`research_military`, `Diplomacy`, `Intrigue`, `Loyalty`.
4. [x] Armées alliées de la province dans la bataille (auto-résolution + `battle_setup`).
5. [x] Événements : capture (+ rançon), effets différés (chaînes), Charles VI + Jeanne de Bourbon, folie.
6. [x] `docs/status.md` (Limites connues) + paragraphes « F1 » des specs.

## Choix (point 1)
- Ciblage : un effet avec `class` ne touche que cette classe (`EffectTotals::classes`), un effet avec
  `unit_category` que cette famille d'unités (`EffectTotals::unit_categories`) ; les champs de premier
  niveau ne gardent que les effets non ciblés. Le pont expose `effects.by_class`.
- `Garrison` N : la ville paie 10 % × N de l'entretien de sa garnison (plafond 50 %) — une première version
  « N unités gratuites » divisait l'entretien des garnisons par deux et l'IA sur-recrutait (banqueroutes) ;
  la garnison d'une ville tenue par son propriétaire et non assiégée regagne 5 % × N de son effectif max par
  saison (plafond 50 %).
- Rééquilibrage : les effets ciblés (richesse des bourgeois, des paysans…) s'appliquaient à toutes les
  classes et gonflaient la richesse ; le ciblage faisait perdre ≈ 25 % de revenu en 10 ans. Richesse de base
  +8 par classe (`CLASS_TARGETING_WEALTH_OFFSET`) : richesse à 10 ans identique à l'avant-F1 (±2).
- `RecruitCost` : province (bâtiments + gouverneur) + techs de la faction, global ou par famille ;
  pourcentages additionnés, plancher 25 % du prix de base.
- `Supply` : en territoire ami, bâtiments de la province + `Supply` fixe du général ajoutés à la
  récupération (40) ; hors territoire ami, `Supply` % et `AttritionResistance` du général réduisent la perte.

## Choix (point 2)
- `army_upkeep` : % global ou par famille sur l'entretien des armées et garnisons.
- `army_experience` : expérience initiale des recrues = `ArmyExperience` fixe des bâtiments de la province
  (buttes de tir, armurerie), du gouverneur et des techs (compagnies permanentes +2), plafond 10.
- `movement` : allure de la famille la plus lente ; les engins de siège ralentissent de 20 %
  (`SIEGE_TRAIN_PACE_PERCENT`, 3 → 2 PM), l'artillerie de campagne (+20 % siège) compense ; `Movement`
  fixe du général (amiral, chevauchée) et des techs ajouté. Appliqué au début de chaque tour et à la
  création d'une armée.
- `production` : % sur la part fiscale des paysans et bourgeois, dont la moitié revient au fisc
  (`PRODUCTION_TAX_SHARE`, sinon la France dépassait 30 000 livres).
- `siege_resistance` : bâtiments + techs positives du défenseur + techs négatives de l'assiégeant
  (0-80 %), réduit la brèche par tour.
- `fortification_level` (maçonnerie) : +1 aux villes déjà fortifiées seulement.
- `wealth` et effets ciblés par classe des techs appliqués à la population.
- `prestige` : chaque hiver, le souverain gagne (bâtiments contrôlés + techs + ses traits) / 5.
- Surplus de recherche : gardé dans `research_progress` tant qu'aucune recherche ne tourne, reporté sur la
  suivante par `start_research` (pas de nouveau champ de sauvegarde).

## Choix (point 3)
- `research_civil` / `research_military` : effets du souverain appliqués aux points de recherche quand la
  recherche en cours est de la branche.
- `Diplomacy` : attitude de A envers B + 2 × diplomatie du souverain de B (±20), raison « Diplomatie de son
  souverain » ; l'acceptation des propositions passe par l'attitude.
- `Intrigue` (choix : chance de capture) : chance de capturer un général vaincu = 10 % + 2 points par point
  d'intrigue d'écart entre le général vainqueur et le vaincu (0-50 %) ; `Side::general_intrigue`.
- `Loyalty` : cible de loyauté d'un vassal + loyauté du souverain vassal + celle du suzerain (±20 chacune) ;
  dans une province, la `Loyalty` des bâtiments (château) et du gouverneur réduit le mécontentement de la
  noblesse.

## Choix (point 4)
- Coalition (`movement::battle_coalition`) : l'armée de la rencontre puis les autres armées de la province,
  de la même faction ou alliées, en guerre contre la faction adverse (ordre des identifiants).
- Auto-résolution : régiments concaténés (chacun avec les techs de sa faction), général commandant = meilleur
  commandement (`coalition_commander`), ravitaillement moyen pondéré par l'effectif ; pertes réparties par
  régiment sur chaque armée ; seul le commandant peut être capturé ; tous les vaincus retraitent.
- `battle_setup` / `resolve_pending_battle` : une armée combinée (`coalition_army`) de même ordre ; le camp
  garde l'id et la faction de l'armée de tête. La bataille est différée si le joueur est dans l'une des
  coalitions (même comme allié). Les assauts de siège restent à deux (armée assiégeante contre garnison).
- `CharacterState::captor` (serde par défaut) : faction qui détient un captif (bataille, chronique).

## Choix (point 5)
- Effets `capture_character { id, faction?, captor }` (le captif quitte armée et gouvernement, un souverain
  garde sa couronne), `release_character { id, faction?, ransom }` (rançon versée au geôlier `captor`, même à
  découvert ; trait « racheté »), `schedule_event { event, delay }` (≥ 1, pas d'auto-programmation), `marry
  { a, b }` (mariage historique). Nouvelle catégorie `chained` : ne se déclenche que programmée (avertissement
  si jamais programmée). `ChronicleState::scheduled` (serde par défaut). Un événement programmé au tour N
  avec un délai k part avec la fin du tour N + k, conditions vérifiées alors, pour la même faction/province.
- Poitiers capture le souverain (Jean II) ; l'option des États programme « La rançon du roi Jean » (+4).
  Brétigny : condition inchangée (captivité de Jean ou, en repli, Poitiers a eu lieu) ; signer la paix
  libère Jean contre 40 000 livres (payées seulement s'il est encore captif ; remplace l'ancien transfert).
- Jeanne de Bourbon (naissance 1338 sans parents modélisés : naissance inconditionnelle, nouveau cas de
  `resolve_births`), « Les noces du dauphin » (1350, effet `marry`), Charles VI (1368, fils de Charles V et
  Jeanne si mariés et vivants). La folie vise `chr_charles_vi` (condition : il règne, 18-40 ans).
- `tools/tests/test_events_schema.py` : ajout de la clé `chained` (ajustement minimal indispensable).

## Vérifications
- Sonde IA (`ai_probe`, graines 1-5, 100 tours) : 0 % d'ordres refusés côté France ; banqueroutes totales
  64 en moyenne contre 79 avant F1, batailles 39 contre 35, provinces prises 16 contre 21 (sièges plus longs :
  résistance, maçonnerie) ; France jamais en banqueroute.
- `./core/build.sh` puis smoke Godot : OK.

## Prochaine étape
Lot terminé : fusion par l'orchestrateur.
