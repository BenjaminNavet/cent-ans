# Lot F1 « Règles inertes » — état

Branche : `worktree-agent-ae66d663ee24e14ab`. Tests : `core/crates/sim-campaign/tests/f1_effects.rs`.

## Points
1. [x] Bâtiments : `Garrison`, `RecruitCost`, `Supply`, ciblage par classe (et par catégorie d'unité).
2. [x] Technologies : `army_upkeep`, `army_experience`, `recruit_cost`, `movement`, `production`,
   `siege_resistance`, `fortification_level`, `wealth`, `prestige` ; surplus de recherche conservé.
3. [x] Traits/compétences : `research_civil`/`research_military`, `Diplomacy`, `Intrigue`, `Loyalty`.
4. [x] Armées alliées de la province dans la bataille (auto-résolution + `battle_setup`).
5. [ ] Événements : capture (+ rançon), effets différés (chaînes), Charles VI + Jeanne de Bourbon, folie.
6. [ ] `docs/status.md` (Limites connues) + paragraphes « F1 » des specs.

## Choix (point 1)
- Ciblage : un effet avec `class` ne touche que cette classe (`EffectTotals::classes`), un effet avec
  `unit_category` que cette famille d'unités (`EffectTotals::unit_categories`) ; les champs de premier
  niveau ne gardent que les effets non ciblés. Le pont expose `effects.by_class`.
- `Garrison` N : les N unités de garnison les plus chères ne coûtent rien ; la garnison d'une ville tenue
  par son propriétaire et non assiégée regagne 5 % × N de son effectif max par saison (plafond 50 %).
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

## Prochaine étape
Point 5 (capture, rançon, effets différés, Charles VI).
