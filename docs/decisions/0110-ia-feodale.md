# 0110 — IA féodale : points d'appel du cœur, ordres féodaux, alliances d'un vassal (chantier FE, lot F5)

Date : 2026-09-28. Statut : accepté. Spec : `docs/superpowers/specs/2026-09-28-feodalite-design.md` § 4.2-4.4, 4.6, 5.
(0101 à 0109 sont pris ou réservés par d'autres chantiers.)

## Contexte

F2 et F3 avaient laissé dans le cœur des scores provisoires (protection, arbitrage) et un seuil de
loyauté unique pour l'ost. Ces décisions sont **réactives** : elles surviennent pendant la résolution
de l'ordre d'une autre faction (une déclaration de guerre appelle le suzerain de la cible, qui convoque
son ost…), là où aucun planificateur ne tourne. Or `crates/ai` dépend de `sim-campaign`, pas l'inverse.
Les autres décisions (commise, concession, révolte, hommage, titre exigé à la paix) n'avaient ni ordre
ni décideur. Enfin le test G4 (Brabant allié de l'Angleterre) était désactivé depuis que les princes
d'Empire sont vassaux déduits de l'Empire.

## Décision

1. **Politique féodale installable** : `sim_campaign::feudal::FeudalPolicy` (pointeurs de fonctions :
   protection, arbitrage, réponse à l'ost, marge d'incertitude, révoltes planifiées). Le cœur garde les
   règles provisoires de F2 (`PROVISIONAL`, IA minimale et tests du cœur) ; `ai::feudal::install()`
   enregistre une fois pour tout le processus (`OnceLock`) les scores de `crates/ai`, appelée par
   `ai::plan_turn`. Fonctions pures de l'état : le déterminisme est préservé.
2. **Décisions proactives = ordres** du planificateur (`ai::feudal::plan_feudal`) : `DeclareCommise`,
   `GrantTitle`, `Revolt`, `SwitchAllegiance`. Le titre exigé à la paix est ajouté au traité du
   vainqueur (`demand_titles`) ; la doctrine « survie d'abord » filtre les déclarations de guerre
   (`filter_suicidal_wars`).
3. **Poids** dans `data/ai/feudal.json` (schéma `ai_feudal.schema.json`), infléchis par
   `ai_personality` (agressivité, diplomatie) ; doctrine par rang dans `data/ai/doctrines.json`
   (`rank_strategies.county = survival_first`).
4. **Révoltes des vassaux IA** : décidées par l'ordre `Revolt` (rapport de forces, tempérament) ; le
   tirage aléatoire de la phase diplomatique ne frappe plus que les liens du joueur. Le cas de félonie
   est ouvert *avant* de rompre le lien (auparavant il ne s'ouvrait jamais, le vassal ne relevant plus
   du suzerain).
5. **Un vassal peut s'allier hors de son suzerain** (`foreign_alliance.allowed`) : un prétendant peut
   courtiser le vassal d'un seigneur qui n'est ni son ennemi ni l'allié de ses ennemis (Brabant,
   Hainaut, Gueldre aux côtés d'Édouard III en 1337-1339). S'allier à l'ennemi de son suzerain reste
   une félonie (`detect_enemy_alliances`, « allié de l'ennemi »), que le suzerain peut sanctionner par
   la commise. Test G4 réactivé.

## Conséquences

- Un binaire de test de `crates/ai` qui interroge les décisions féodales sans passer par `plan_turn`
  appelle `ai::feudal::install()` d'abord (sinon, selon l'ordre des tests, il verrait les règles
  provisoires).
- Le pont Godot appelle `ai::feudal::install()` à la création de `CampaignSim` : l'aperçu d'escalade
  affiché au joueur et les décisions de l'IA suivent les mêmes scores.
- Les ordres féodaux sont réutilisables par l'interface (F6).
- L'équilibre (fréquence des commises, révoltes, hommages) reste à juger en F8 sur parties longues.
