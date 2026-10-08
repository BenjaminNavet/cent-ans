# FE — Féodalité et petites factions jouables (conception)

Date : 2026-09-28. Chantier FE (lots F0-F8). ADR associé : `docs/decisions/0098-titres-au-dessus-des-factions.md` (à écrire en F0).

## 1. But

Donner à chaque joueur un **départ plus petit** qui laisse une marge de progression, **sans fausser
les frontières historiques de 1337**. Moyen : découper les royaumes selon leur réalité féodale en
vassaux-factions jouables. Le royaume garde ses frontières exactes ; le domaine direct du joueur
royal devient petit ; la progression consiste à intégrer ses vassaux (ou, pour un vassal, à
s'émanciper, s'élever ou usurper).

Hors périmètre : l'extension de la carte jusqu'à l'Oural et les factions de l'Est (spec suivante,
qui réutilisera ce modèle).

## 2. Décisions validées

| Sujet | Décision |
|---|---|
| Ordre des chantiers | Féodalité sur la carte actuelle d'abord ; Oural ensuite |
| Autonomie des vassaux | Factions à part entière : trésor, armées, IA, diplomatie propres |
| Étendue | Tous les royaumes découpés selon 1337 ; toutes les factions jouables (sauf `fac_rebels`) |
| Absorption | Commise pour félonie, déshérence/héritage, mariage, conquête (pas d'intégration pacifique « EU4 ») |
| Buts des petites factions | 2-3 objectifs historiques par faction + voies génériques (indépendance, ascension, couronne) |
| Hiérarchie | Plusieurs niveaux, **3 au plus** (royaume, duché, comté) ; suzeraineté par province possible |
| Maxime | « Le vassal de mon vassal n'est pas mon vassal » : on ne traite qu'avec son suzerain direct et ses vassaux directs |
| Guerre | Escalade maillon par maillon, chaque suzerain décide ; guerre privée arbitrée par le suzerain commun |
| Volume | ≈ 130 factions, ≈ 250 provinces |
| Architecture | Titres au-dessus des factions (approche 1) |

## 3. Modèle de données

### 3.1 Titres — `data/titles/<id>.json`, schéma `data/schemas/title.schema.json`

- `id` (`tit_*`), `rank` : `kingdom` | `duchy` | `county`. L'Empire est un `kingdom`.
- `name` (`display`, `local`, `local_language`), `heraldry` (même forme que les factions).
- `de_jure_liege` : titre dont il relève ; absent pour un royaume souverain. Un titre relève
  toujours d'un titre de rang strictement supérieur.
- `de_jure_provinces` : provinces propres du titre (domaine direct *de jure*). La couverture totale
  d'un titre est l'union de ses provinces propres et de celles de ses titres vassaux.
- `holder_1337` : faction détentrice au départ, avec `sources` et `uncertain` comme les données actuelles.
- `objectives` : 2-3 objectifs historiques (voir 4.8), portés par le titre principal d'une faction.

Invariants (validés par `tools/` et au chargement) : pas de cycle, 3 niveaux au plus, chaque
province appartient à exactement un titre de rang `county` ou au domaine propre d'un titre
supérieur, un `holder_1337` par titre.

### 3.2 Changements de l'existant

- **Faction** : ajout de `primary_title` ; `playable` vaut `true` par défaut (`fac_rebels` exclu).
- **Province** : `owner` = faction qui gère la province (inchangé). `overlord` et `holder` sont
  **supprimés** : ils se déduisent des titres. Script de migration dans `tools/` pour les données actuelles.
- **Personnages** : un souverain par faction ; héritier présomptif et liens matrimoniaux existants réutilisés.

### 3.3 Déductions (calculées par le cœur, jamais stockées)

- Suzerain d'une faction = détenteur du `de_jure_liege` de son titre principal (aucun si c'est
  elle-même ou si le titre est souverain).
- Suzerain pour une province = détenteur du titre supérieur qui la contient. D'où la double
  allégeance : Édouard III tient le royaume d'Angleterre (souverain) et le duché de Guyenne (relevant de France).
- Arbre féodal = graphe des titres détenus.

### 3.4 Volume

≈ 130 factions, ≈ 250 provinces. Les subdivisions portent surtout sur la France, l'Empire et les
Pays-Bas. La géométrie vient du pipeline existant `cent-ans geo provinces` (graines + poids),
alimenté par les nouvelles graines. Les colonies (`data/settlements`) sont réaffectées aux provinces subdivisées.

## 4. Règles du cœur

Module `core/crates/sim-campaign/src/feudal.rs`. Paramètres dans `data/rules/feudal.json`.
Aucune valeur codée en dur.

### 4.1 Obligations
- Le vassal verse à son suzerain **direct** une part de ses revenus (défaut 10 %).
- Le vassal doit l'**ost** quand son suzerain direct entre en guerre.
- Le suzerain doit **protection** à ses vassaux directs (voir 4.3).

### 4.2 Loyauté (0-100, du vassal envers son suzerain direct)
- En hausse : protection accordée, liens familiaux, culture commune, rapport de forces favorable
  au suzerain, octroi de titres.
- En baisse : protection refusée, taxe, commise frappant un pair, défaites du suzerain, prétendant rival.
- Sous `feudal.json:disloyal_threshold`, le vassal peut refuser l'ost, se révolter (guerre
  d'indépendance) ou prêter hommage à un autre suzerain de rang suffisant.

### 4.3 Escalade de guerre
1. L'attaque d'une faction appelle son suzerain direct à la protéger.
2. Il **intervient** (entre en guerre et convoque l'ost de ses propres vassaux directs) ou **se dérobe**
   (perte de prestige, baisse de loyauté de tous ses vassaux directs ; le vassal attaqué peut changer d'allégeance).
3. S'il intervient, son propre suzerain est appelé à son tour, selon le même choix.
4. La décision de l'IA pondère : rapport de forces, relations, trésor, guerres en cours, personnalité.
5. **Guerre privée** entre deux vassaux d'un même seigneur : ce seigneur arbitre (imposer la paix,
   prendre parti, laisser faire).

La maxime s'applique partout : un suzerain ne lève jamais directement les arrière-vassaux.

### 4.4 Félonie et commise
- Refus d'ost, alliance avec l'ennemi du suzerain ou révolte ouvrent un **cas de commise** pendant
  `feudal.json:felony_window_turns` tours.
- Le suzerain peut prononcer la commise : c'est un casus belli contre le seul félon, dont les
  vassaux sont appelés selon 4.3.
- En cas de victoire, le titre revient au suzerain, qui le garde ou le concède à un fidèle.

### 4.5 Héritage et mariage
- À la mort d'un souverain, chaque titre passe à l'héritier selon la loi de succession existante.
- Si l'héritier dirige une autre faction : fusion des titres (union personnelle).
- Sans héritier : le titre revient au suzerain (déshérence).
- Plusieurs prétendants : guerre de succession, arbitrée par le suzerain.
- Le mariage modifie les droits de succession via les liens matrimoniaux existants.

### 4.6 Conquête
Après une victoire, le vainqueur peut exiger un titre : il l'usurpe s'il est de rang égal ou
supérieur, sinon il le concède à l'un de ses vassaux directs.

### 4.7 Titre suzerain vacant
Si un souverain perd son dernier titre de rang supérieur, ses vassaux directs deviennent indépendants.

### 4.8 Objectifs et victoire
- Objectifs historiques (données), évalués chaque tour : types `hold_title`, `hold_provinces`,
  `be_independent`, `be_liege_of`, `hold_crown`. Exemples : Foix réunit Foix et Béarn et tient le
  Béarn souverain ; Navarre recouvre ses terres normandes.
- Victoire : tous les objectifs historiques atteints, **ou** une voie générique :
  - indépendance tenue `feudal.json:independence_turns` tours ;
  - premier vassal du royaume : plus forte puissance (provinces, armées, trésor) parmi les vassaux directs du souverain pendant `feudal.json:ascension_turns` tours ;
  - couronne du suzerain obtenue.

## 5. IA, performances, équilibre

- **IA** (`crates/ai`) : décisions de répondre à l'appel de protection ou d'ost, de prononcer la
  commise, de concéder, de se révolter ou changer d'allégeance, d'arbitrer. Chaque décision est un
  score pondéré par `ai_personality` et `data/ai/feudal.json`. Doctrine « survie d'abord » pour les
  petites factions (alliances, mariages, pas d'expansion suicidaire).
- **Performances** : fin de tour au plus 1,5 fois le temps actuel, mesuré par le banc `pb1`. Si le
  budget est dépassé : évaluation allégée un tour sur deux pour les factions sans armée levée ni guerre.
- **Équilibre** : la guerre de Cent Ans éclate à toutes les difficultés (ADR 0085), déclenchée par
  la commise de la Guyenne. Parties IA contre IA de 100 ans sur N graines, avec bandes cibles :
  - la France reste le royaume le plus peuplé ;
  - elle absorbe une partie de ses grands fiefs sur la durée, sans tout avaler en 20 ans ;
  - des guerres de succession surviennent ;
  - aucune faction mineure n'explose.
- **Sauvegardes** : nouvelle version de format ; une ancienne sauvegarde affiche un message clair, sans migration.

## 6. Interface (Godot, rendu seulement)

- **Choix de faction** : carte cliquable ; fiche au survol (souverain, titres, suzerain,
  difficulté estimée, objectifs) ; filtres par royaume et par rang ; six départs recommandés
  (France, Angleterre, Bourgogne, Bretagne, Flandre, Navarre).
- **Filtre de carte « Féodalité »** (filtres MF1) : fond du royaume, hachures du tenant direct,
  écu parti pour une double allégeance.
- **Fiche de province** : fil d'Ariane cliquable (`Royaume de France › Duché de Bourgogne › Comté de Charolais`).
- **Panneau « Arbre féodal »** repliable, ouvert sur la position du joueur : pastilles d'état
  (loyal, mécontent, félon, en révolte), loyauté au survol, actions (commise, concession, arbitrage).
- **Obligations** affichées dans le panneau de faction (taxe, ost, vassaux à protéger).
- **Panneau « Qui peut entrer en guerre »** avant toute déclaration : chaîne d'escalade et
  estimation par maillon (probable, incertain, improbable), avec la raison principale.
- **Événements** : appel de protection, appel d'ost, cas de commise, mort sans héritier, guerre de succession.
- **Objectifs** : suivi dans le panneau de faction.
- **Codex** : entrée « Vassalité » ; tutoriel de trois étapes au premier lancement.

## 7. Données, coûts, tests

- **Registre 1337** rédigé par royaume (France ; Empire et Pays-Bas ; Îles britanniques ; Ibérie ;
  Italie), sources citées et incertitude marquée. Relecture par un agent historien (anachronismes)
  à chaque lot, sur le modèle de `docs/archive/chantiers.md`.
- **Portraits** : ≈ 170 images (≈ 110 souverains, ≈ 60 héritiers) à ≈ 0,045 $, soit ≈ 8 $, plus
  les variantes âgées des souverains de plus de 50 ans. **Plafond propre FE : 15 $**, consigné dans `docs/budget.md`.
- **Armoiries** : générées depuis le blason par le pipeline DA1, sans coût.
- **Tests** :
  - Rust (`sim-campaign/tests/feudal_*.rs`) : déductions (suzerain, double allégeance, maxime),
    escalade, commise, héritage, union personnelle, déshérence, guerre privée, objectifs ;
  - Python : validation du registre (invariants de 3.1) ;
  - Godot : `smoke.gd` étendu (filtre Féodalité, arbre, choix de faction), une capture de preuve par écran ;
  - Équilibre : bandes cibles de la section 5, sans régression de l'ADR 0085.

## 8. Lots

| Lot | Contenu | Dépend de |
|---|---|---|
| F0 | Squelette (schéma, `feudal.rs`, tests désactivés), ADR 0098, migration des données actuelles vers les titres à provinces constantes | — |
| F1 | Déductions, obligations (taxe, ost), loyauté | F0 |
| F2 | Escalade, guerre privée, données du panneau « Qui peut entrer en guerre » | F1 |
| F3 | Commise, héritage, mariage, conquête, titre vacant, objectifs | F1 |
| F4 | Subdivision des provinces et registre complet, par royaume (lots parallèles) | F0 |
| F5 | IA féodale | F2, F3 |
| F6 | Interface | F2, F3 (F4 pour le choix de faction) |
| F7 | Portraits | F4 |
| F8 | Équilibre et partie pilote | tous |
