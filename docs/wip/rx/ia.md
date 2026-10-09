# RX — expert IA de campagne

Périmètre : `core/crates/ai`, `data/ai/*.json`, `data/rules/difficulty.json`, ADR 0003 / 0148.
Mesures du 2026-10-09, machine très chargée (load average 30 à 96, autres sessions). Aucun fichier du dépôt modifié ; la sonde jetable est hors dépôt (`/private/tmp/claude-501/.../scratchpad/iaprobe`, mesures dans `/private/tmp/claude-501/rx-shots/ia/`).

## 1. Verdict

- Forces : IA déterministe (pas de `HashMap`, tirages salés par graine/tour, test parallèle = séquentiel), rapide au CPU (4 ms médiane par faction), stable (oscillations rarissimes), `cargo test -p ai` vert (105 tests), une seule guerre « éternelle » sur 5 000 guerres, et c'est une guerre scénarisée.
- Faiblesse majeure 1 : la difficulté ne change que des chiffres (revenu, entretien, moral) ; l'IA joue exactement pareil à tous les niveaux.
- Faiblesse majeure 2 : l'outil de duel A/B de l'ADR 0148 n'existe plus (supprimé par `c2308153d`) alors que l'ADR dit qu'il « reste dans le jeu » ; plus aucun moyen de juger une règle de campagne, et `docs/wip/ia-nuit.md` est introuvable.
- Faiblesse 3 : 60 % des sièges ne finissent pas par une prise, et 15 % des guerres sont redéclarées dans les 12 tours qui suivent la paix (ping-pong paix/guerre).
- Le temps de tour mesuré à l'horloge murale est inexploitable sur cette machine (voir constat 6) ; au temps CPU il est bon.

## 2. Constats

### [majeur] [conception] La difficulté n'agit pas sur l'intelligence de l'IA
**Constat** : le joueur en « Très difficile » affronte la même IA qu'en « Facile », avec des bonus de revenu (+40 % / −20 %), d'entretien, de moral et d'hostilité. Les trois leviers qui rendent l'IA moins prévisible ou plus sournoise (agressivité, ratio de guerre, mercenaires, sièges) ne bougent pas, sauf `ai_war_ratio_percent`. Une IA « facile » qui a 80 % de revenu mais joue pareil reste un adversaire cohérent, et la « très difficile » n'est qu'une IA riche.
**Preuve** : `core/crates/data-model/src/entities/difficulty.rs:10-29` (huit champs, tous numériques) ; `data/rules/difficulty.json` ; `grep difficulty core/crates/ai/src` ne donne aucun résultat (l'IA ne lit pas le niveau). Les cibles EQ6 (guerre FR-EN 55-75 %) tiennent à tous les niveaux, ce qui confirme que l'IA ne s'adapte pas.
**Correction proposée** : ajouter 2-3 champs à `DifficultyModifiers` lus par `ai::campaign` : `ai_siege_superiority_percent` (ratio exigé avant de sièger, `siege_superiority`), `ai_assault_odds_delta` (seuil d'assaut), `ai_mercenary_rich_percent`. Données dans `data/rules/difficulty.json` + schéma. Mesurer au duel (constat 2) avant de fixer les valeurs.
**Coût** : M.

### [majeur] [bug de processus] L'outil de mesure A/B de l'ADR 0148 a disparu
**Constat** : toute modification de règle de campagne est désormais aveugle ; l'ADR 0148 impose le duel A/B et l'ADR indique que `ai::experiment` « reste dans le jeu ». Les notes `docs/wip/ia-nuit.md` n'existent plus non plus.
**Preuve** : `git log --diff-filter=D` : `c2308153d chore(sc-probes): remove throwaway Rust probes, examples and the experiment toggle module` ; `ls core/crates/ai/examples` ne contient plus que `pb1_core`, `playthrough`, `turn_digest`, `turn_hotspot`, `turn_perf` ; `century_probe`/`balance_probe`/`ia_quality_probe` aussi absents (cités dans EQ6 et ADR 0148).
**Correction proposée** : (a) ajouter une note d'ADR « amendement » à 0148 disant que l'outil est retiré et comment le restaurer (`git show c2308153d^:core/crates/ai/src/experiment.rs`) ; (b) mieux, conserver la sonde de la présente revue comme exemple `ai/examples/campaign_health.rs` : guerres déclarées/terminées, redéclarations rapides, sièges prises/levés, oscillations ABAB, temps de tour. Elle tient en 90 lignes et ne dépend que de l'API publique.
**Coût** : S (a), S-M (b).

### [majeur] [équilibrage] 60 % des sièges se terminent sans prise
**Constat** : sur 4 graines × 464 tours, 931 sièges commencés, 369 se terminent par la prise (contrôleur = assiégeant) et 562 autrement (siège levé, paix, déroute). Le joueur voit des armées qui campent des années devant une ville puis s'en vont. Non distingué par la sonde : levée de siège par paix (légitime) vs abandon volontaire (`give_up_hopeless`) vs armée détruite.
**Preuve** : sonde, par graine (début / pris / autre) : 290/115/175, 266/108/157, 141/51/89, 234/95/138. Réglages : `data/ai/campaign.json` `siege_patience_turns: 12`, `assault_odds: 65`, `siege_hold_threat_factor: 1.2` ; `core/crates/ai/src/campaign/army.rs:375-395` (`give_up_hopeless`) et `:483-507` (`keep_siege`). Pas de mémoire de cible : `pick_siege` (`army.rs:546`) recalcule chaque tour, sans hystérésis.
**Correction proposée** : journaliser la raison de fin de siège dans la sonde (paix / abandon / armée détruite / secours) avant de toucher aux seuils ; si « abandon » domine, relever `siege_patience_turns` ou conditionner `give_up_hopeless` au fait qu'une autre cible attende vraiment l'armée.
**Coût** : S (diagnostic) puis S-M.

### [majeur] [équilibrage] Guerres rejouées juste après la trêve
**Constat** : 15 % des guerres (213/1372, 187/1213, 204/1348, 228/1415) sont redéclarées par la même paire en 12 tours ou moins après la paix ; la trêve ne dure que 8 tours. Du point de vue du joueur : « paix » incohérente, des royaumes qui se réconcilient puis se battent de nouveau à l'expiration de la trêve.
**Preuve** : sonde ; `data/ai/diplomacy.json` `negotiation.peace_truce_turns: 8`, `min_war_turns: 20`. Durée médiane des guerres terminées : 20 tours sur les 4 graines, soit exactement `min_war_turns` : le plancher décide, pas la situation.
**Correction proposée** : trêve plus longue quand la paix suit un score de guerre net (par ex. 8 + 1 tour par 10 points de score, plafond 20), ou pénalité d'attitude décroissante après une paix. À mesurer : `fast-redeclare` doit passer sous 5 % sans toucher la bande FR-EN d'EQ6.
**Coût** : S pour le test d'une trêve plus longue, M pour la règle.

### [mineur] [conception] Pas de hystérésis des objectifs d'armée
**Constat** : chaque armée recalcule sa cible chaque tour (valeur / (1 + distance/2)), seul le siège en cours est tenu. Mesuré, l'effet visible est faible : motif ABAB sur 3 positions de rattachement en 6 tours chez 14-21 armées par partie, 42 à 136 tours-armée sur 44 000 (≈ 0,1-0,3 %). Pas de comportement absurde général, mais cas résiduels qui se voient sur la carte vivante (CV3).
**Preuve** : sonde ; `army.rs:277-298` (`plan_army`, aucun état conservé d'un tour à l'autre).
**Correction proposée** : bonus de continuité de ~15 % sur la cible précédente (`Army.destination` existe déjà dans `state.rs:287`) dans `pick_siege`/`pick_defence`. Ne le faire qu'avec le duel du constat 2.
**Coût** : S-M.

### [mineur] [bug de mesure] Le temps de tour à l'horloge murale ne dit rien ici
**Constat** : tour complet (175 factions, joueur inclus), même graine : médiane 0,67 s lors de la première mesure, 2,9 s lors de la seconde, p95 5 s et max 22 s sur des graines tardives. Cause : charge machine 30-96. Au temps CPU de thread (`turn_perf`), le coût est : moyenne 5,9 ms, médiane 4,2 ms, p95 15 ms, p99 29 ms par faction (6 998 tours de faction, 40 tours) ; pire cas : Hongrie 301 ms au tour 1, Holstein 259 ms, Berg 138 ms, Angleterre 24,8 ms de moyenne.
**Preuve** : `/private/tmp/claude-501/rx-shots/ia/perf1.txt`, `slow3.out`/`long1.out`. Cible du design : 50 ms par faction (`turn_perf.rs:3-5`) : tenue sauf au tour 1 pour Hongrie/Holstein/Berg.
**Correction proposée** : lancer `turn_hotspot 1 1 fac_hungary` sur machine calme pour voir pourquoi le premier tour coûte 6 fois la cible (cache froid de `GridPlanner` ou routes mises en cache ?). Pour la fluidité, un indicateur de progression « les IA jouent » côté Godot suffit tant que le tour reste sous 1-2 s.
**Coût** : S (mesure) à M (optimisation).

### [mineur] [conception] Une guerre dure tout le jeu : croisés contre mamelouks
**Constat** : sur la graine 1, la seule guerre de plus de 150 tours est `fac_crusaders` vs `fac_mamluks`, depuis le tour 0 jusqu'au tour 464 ; sur les graines 2-4 seules des guerres de < 400 tours subsistent (la plus longue : 363 tours, graine 3). C'est cohérent avec la fiction (croisade), mais les armées de ces deux camps (2 armées en fin de partie) ne mènent plus aucune action.
**Preuve** : sonde (`LONGWAR` graine 1). `docs/wip/jr-croises.md` pour l'intention.
**Correction proposée** : aucune, à surveiller ; si la croisade doit vivre, vérifier la logique Fervor, pas les règles de paix.
**Coût** : S.

### [mineur] [finition] Mémoire de décision : `docs/wip/ia-nuit.md` référencé mais absent
**Constat** : ADR 0148 et la mémoire projet renvoient à `docs/wip/ia-nuit.md`, absent du dépôt.
**Preuve** : `ls docs/wip | grep ia` ne renvoie rien d'utile.
**Correction proposée** : corriger le renvoi dans l'ADR 0148 (vers l'ADR lui-même ou vers `docs/wip/archive/`).
**Coût** : S.

## 3. À ne surtout pas changer

- La structure de planification pure `plan_turn(state, data, faction)` et le mode séquentiel/parallèle (`parallel.rs`) : déterminisme garanti par `pb3f_parallel_plan` (vert).
- L'absence de `HashMap`/`SystemTime` dans la crate et les tirages salés (`salts.rs`) : c'est ce qui rend les empreintes `turn_digest` comparables.
- Les réglages `data/ai/diplomacy.json` d'EQ6 (`claim_war_ignores_kinship`, `weary_stay_out`) : bande FR-EN tenue à tous les niveaux.
- Le rythme global des guerres : 3 par tour sur 177 factions, durée médiane de 5 ans, 113-127 factions vivantes sur 177 au bout de 464 tours, aucune partie dégénérée.
- La séparation « IA de campagne dans `ai`, IA minimale dans `sim-campaign` » (ADR 0003).
