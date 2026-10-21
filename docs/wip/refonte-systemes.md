# Refonte des systèmes : mêlée, révoltes, économie, IA de campagne

État au 21/10/2026 : **plan seulement, rien n'est implémenté.** Quatre chantiers indépendants,
à lancer dans une autre session. Chacun commence par un lot de **mesure** (seed bots, ADR 0348,
ou tests), sans changement de règle, puis enchaîne des lots de règles. Le détail de chaque lot
(fichiers, tests, risques) est dans les notes de travail locales de l'agent.

Ordre conseillé : **révoltes**, puis **mêlée**, puis économie et IA (qui touchent tout
l'équilibre ; les faire avec les mesures en main).

## 1. Révoltes et ordre public

Constat : ≈ 2,7 révoltes par partie, sous la cible. Modèle actuel (`core/` population,
`data/rules/population.json`) : mécontentement par classe, révolte si moyenne > 75 pendant
5 saisons d'affilée ; compteur remis à zéro à chaque interruption et après chaque révolte ;
une révolte coûte 25 % de garnison, la ville ne passe aux rebelles qu'au-dessus de 90,
sans armée ni bataille. Moins d'occupations depuis la règle de fatigue d'EQ6.

- **A — mesure** : instrumenter les révoltes dans les seed bots (nombre, durée au-dessus du
  seuil, province occupée ou non). Unifier la bande cible : 3-15 (seed bots) contre 4-10
  (`balance_probe`). Corriger la description de `population.json` (« 2 saisons » alors que
  la valeur est 5). L'écart actuel est dans le bruit (±0,8) : mesurer avant de conclure.
- **B** : jauge de tension cumulée (monte au-dessus du seuil, redescend lentement) à la
  place du compteur de saisons consécutives.
- **C** : armée rebelle levée selon la richesse et la culture de la province ; la vaincre
  (répression) rend de l'ordre.
- **D** : choc d'ordre public ponctuel selon le sort réservé à une ville prise
  (occuper, piller, raser).
- E (optionnel) : paliers d'humeur avec effets visibles.
- F (optionnel) : contagion aux provinces voisines.

## 2. Mêlée, moral, déroute, fatigue

Suite de l'ADR 0711 (`docs/wip/choc-charges.md` § Suite) : seuil d'élan, plancher d'élan,
immunité au choc et déclin du bonus de charge sont codés mais **neutres** dans
`data/rules/battle_impact.json`, car l'équilibre actuel repose sur des impacts répétés
(≈ 7 000 par bataille). Tests à surveiller : b6, ai_beats_passive, duel,
Crécy [14,20), Poitiers [11,19).

- **C1 — mesure** : budget d'impacts par bataille et par catégorie, sans changer de règle.
- **C2** : bonus de charge décroissant (fort au pic, ≈ 8 s) à intégrale égale au palier
  actuel de 4 s, donc neutre pour l'équilibre.
- **C5** : fatigue par paliers, qui touche aussi l'armure et le bonus de charge ; en mêlée
  la fatigue doit monter à peu près aussi vite que le repos récupère (aujourd'hui le repos
  récupère ≈ 2,7× plus vite : la mêlée n'use pas).
- **C3** : activer élan (seuil ≈ 5 m, plancher 0,5) et immunité au choc (1 à 4 s),
  compensés par un rythme de mêlée par catégorie.
- **C4** : IA de cavalerie qui se dégage, reprend du champ et recharge.
- **C7** : compteur de déroutes (une unité qui déroute 3 fois ne se rallie plus) et
  fenêtre de fragilité après ralliement.
- **C6** : moral séparant pertes récentes (effet brutal) et pertes totales (effet doux),
  rapport de forces local, chocs de flanc et de dos, mort du général graduée.
- C8 (optionnel, très risqué) : défense distincte de l'attaque, part de dégâts perforante,
  toucher borné par un plancher et un plafond.

## 3. Économie et bâtiments

Écarts : nombre d'emplacements fixe ; entretien par bâtiment (l'IA doit démolir les
bâtiments non rentables) ; presque aucun bâtiment n'allège le coût des armées (2 sur 40) ;
un seul bâtiment agit sur la croissance ; spécialisation seulement souple (vocations) ;
aucune métrique économique dans les seed bots. Voir aussi les chantiers CO et RG.

- **0 — mesure** : métriques économiques dans les seed bots (revenus, dépenses par poste,
  faillites, démolitions).
- **C** : croissance portée par les bâtiments agricoles et portuaires.
- **D** : entretien des bâtiments revu (faible ou nul ; la vraie dépense reste l'armée).
- **B** : réserve de développement par province qui borne les emplacements.
- **A** : emplacements gagnés par palier de taille de la ville.
- **E** : bâtiments qui réduisent le coût et l'entretien des armées.
- **F** : chaînes de ressources et groupes de bâtiments exclusifs.

## 4. IA de campagne

Modèle actuel : 4 scalaires `ai_personality` par faction (lus à 3 endroits), attitude
diplomatique à motifs, deux parts militaires guerre/paix, cibles sans notion de phase de
partie ; aucune mesure de diversité de comportement entre factions.

- **F6 — mesure** : sondes de diversité de comportement dans les seed bots.
- **F1** : personnalités composées de briques réutilisables (cibles, budget, diplomatie),
  avec variantes selon la situation stratégique.
- **F4** : budget par poste avec plafond et report ; vise aussi faillites et révoltes.
- **F2** : mémoire diplomatique à déclin, transmise aux alliés et ennemis.
- **F3** : trahison, fiabilité, facteurs ami de l'ennemi.
- **F5** : cibles par phase de partie et par groupes de régions.

## Prochaine étape

Choisir le ou les chantiers à lancer, et si les lots s'enchaînent sans validation
intermédiaire. Pour chaque chantier : lot de mesure d'abord, ADR si une règle change,
passe rapide des seed bots en fin de lot.
