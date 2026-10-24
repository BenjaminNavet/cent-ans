# 0784 — L'arc de partie en trois actes

Statut : accepté (lot ACTES, 24/10/2026).

## Contexte

Le juge game design (`out/juge/iter-02-design.md`, `iter-03-design.md`) note que les victoires sont des listes de
provinces à tenir (`hold_turns` jusqu'à 40) : pas d'arc de partie, pas de fil rouge, pas de tension qui change entre
1337 et 1453. Chantiers demandés : « le roi et sa dette » (fait, ADR 0777), « arc en trois actes », « fixer le fantasme :
le roi et sa maisnie ».

## Décision

1. **Trois actes, en données.** `data/rules/acts.json` (schéma `acts_rules.schema.json`, types `data_model::ActsRules`) :
   Acte I « La querelle » (1337-1360 : avènement de la guerre, Crécy, Poitiers, Brétigny), Acte II « La reprise et la ruine »
   (1360-1415 : Charles V, schisme, Azincourt), Acte III « Le sursaut » (1415-1453 : Troyes, Jeanne d'Arc, Castillon).
   Poids des actes dans le score de règne : 25, 30, 45.
2. **Objectifs de maison par acte.** `objective_sets` : une clé par faction (`fac_france`, `fac_england`, `fac_burgundy`),
   par gouvernement (`gov_kingdom`, `gov_duchy`... : les génériques) ou `default`. Chaque objectif a une catégorie
   (domaine, trésor, lignée, prestige, alliances), un poids et une condition (`ActCondition` : contrôle de provinces,
   part du domaine initial, prestige, trésor, dette sous un plafond, alliés, lignée assurée — souverain libre, héritier,
   pas de régence —, titre, indépendance, soumission, guerre).
3. **Pressions d'acte.** Conditions permanentes qui changent d'un acte à l'autre et ajoutent ou retirent des points au
   score de l'acte si elles tiennent à la clôture (p. ex. acte I : « La couronne contestée » — l'Anglais tient une province
   du royaume — pour la France, -10 ; acte III : « La reconquête »). Publics visés par faction ou gouvernement.
4. **Politique de la maisnie.** À chaque acte le joueur choisit une politique parmi trois ; elle compte une catégorie
   d'objectifs 150 % (choix modifiable tant que l'acte dure ; non choisie : aucun effet).
5. **Transitions conditionnées à la partie.** Un acte se clôt quand l'année dépasse sa fin (`year`), ou plus tôt par une paix
   durable (`peace` : N saisons sans guerre après une durée minimale) ou un effondrement (`collapse` : moins de la moitié des
   provinces du début de l'acte, ou deux banqueroutes royales dans l'acte). Plusieurs actes peuvent se clore à la suite si le
   calendrier saute. Le bilan (`ActReport`) garde le score de l'acte, les objectifs et pressions, la politique et les trois
   principales causes du journal (ADR 0754, copiées car le journal est borné).
6. **Écran de transition.** `game_api::acts::transition_view` : bilan, causes, acte suivant, choix de politique ; le client
   l'affiche une fois (`acts:policy:<id>`, `acts:continue`). Les textes sont en données.
7. **État sauvegardé.** `CampaignState::acts: ActsState` ; `STATE_VERSION` 10 -> 11, migration `acts` vide, fixture
   `save_v11.json` (les `save_v9.json` et `save_v10.json` restent chargées).
8. **Ruine.** Défaite si : la lignée de la maison s'éteint (gouvernements dynastiques), banqueroute royale irréversible
   (3 banqueroutes, trésor encore négatif), capitale perdue sans héritier pendant 8 saisons — en plus de la perte de toutes
   les terres. La victoire est décidée par l'ADR 0785.

## Conséquences

- Les objectifs de `data/factions/*.json` (`victory`) ne décident plus de la partie lorsque l'arc est actif ; ils restent
  chargés (résumés, `objectives()`), et reprennent la main si `acts.json` est absent ou vide.
- Aucune règle d'économie ni d'IA ne change. Seul le joueur est suivi.
- Limites : les pressions sont des bonus ou malus de score, non des effets de simulation ; la politique ne change que la
  pondération du bilan ; les conditions négatives (dette trop haute) s'expriment par leur contraire.
