# ADR 0068 — L'IA budgète ses engagements et ne campe plus en terre étrangère (lot EQ5)

Date : 2026-09-26. Statut : accepté.

## Contexte

La mesure combinée EQ4 (`docs/wip/eq4-equilibre-combine.md`, addendum de l'ADR 0054) laissait
deux défauts de l'IA de campagne :

1. **Banqueroutes chroniques** de petites IA : Grenade à tous les niveaux (jusqu'à 7,8 / déc.),
   Suisses et Gueldre en facile (Suisses 31 / déc. sur une graine), soit 0,42 / faction /
   décennie en facile. Baisser l'entretien des armées de l'IA (données) n'y changeait rien.
2. **Intrusions** : ~1 300 saisons par siècle (normal) d'armées de l'IA sur les terres d'un
   royaume en paix sans droit de passage (DP2, ADR 0031), 2 200-2 500 en difficile.

Diagnostic (sonde `century_probe` : `ECON_TRACE`, `TRESPASS_TRACE`, `DEBUG_ARMY`) :

- Grenade perdait des guerres lointaines (Angleterre, Écosse, Holstein) et payait des tributs
  par saison que le budget de l'IA ignorait ; les agents et la Table (H3) non plus.
- Les Suisses avaient bâti sur leur trésor initial : après la peste, l'entretien des bâtiments
  mangeait 60-75 % du revenu, sans armée à licencier. L'impôt oscillait Haut / Normal à chaque
  saison autour du seuil de trouble ; l'affaiblissement de la monnaie gonflait pour toujours
  l'entretien des bâtiments (prix 100 → 124).
- 38 % des saisons d'intrusion étaient des armées dans leur propre place (château tenu) d'une
  province étrangère ; les terres fermées bloquaient aussi la **sortie** d'une armée prise là
  par une paix (table de routes vide : elle restait des années) ; l'IA poursuivait des armées
  ennemies réfugiées en terre neutre ; les routes du graphe des colonies mordaient sur des
  provinces fermées (le Véronais sur les routes de la plaine du Pô).

## Décision

Règles d'IA seulement (`core/crates/ai`), aucune règle d'économie ni de passage changée.

**Budget** (`campaign.rs`)

1. Le revenu net de l'IA retranche ses **engagements** : tributs dus et entretien des agents.
   Les rançons et la Table (H3) n'y entrent pas : elles ne se paient que si le trésor le peut
   (premier essai rejeté : compter une rançon faisait licencier à l'Écosse toutes ses garnisons
   en pleine guerre, et elle disparaissait en difficile).
2. Une **marge de sécurité** (5 % du revenu brut) est gardée : un bâtiment n'est lancé que si
   son entretien tient dans l'excédent moins cette marge.
3. L'entretien des bâtiments est **plafonné à 30 % du revenu brut**, sauf ceux qui rapportent
   au moins leur entretien en impôts ou en commerce (ils ne se licencient pas quand les temps
   changent).
4. **Impôt Haut** : une fois levé, il reste tant qu'au taux normal le budget serait en déficit
   et que le trésor tient moins d'une saison de revenu, ou tant que le trésor est en dette ;
   dans ces deux cas le trouble toléré monte de 30 à 40 (le garde-fou « province au bord de la
   révolte » reste). Premier essai rejeté (garder Haut jusqu'à la réserve de 3 saisons) : 44-47 %
   des échantillons en impôt Haut, au-delà de la cible E2 (< 40 %).
5. Pas de monnaie affaiblie quand les bâtiments prennent plus d'un tiers du revenu brut (au
   lieu de la moitié).

**Armées** (`grid.rs`, `campaign.rs`, `diplomacy_eval.rs`)

6. Les places que la faction tient ne sont jamais des terres fermées pour elle ; une route qui
   part de terres fermées peut traverser les terres de ce même maître (la sortie).
7. Une armée sans objectif (défense, siège, chevauchée) en terre étrangère sans droit de passage
   **rentre au pays** : la place à elle la plus proche hors des terres fermées, dans la portée de
   planification puis quatre fois plus loin en traversant les terres fermées (une courte
   intrusion vaut mieux que des années de camp), sinon une place à elle quelconque ; seulement
   une place sur la même terre de la grille (une route par la mer n'est pas un chemin).
8. En paix, une armée dans sa propre place d'une province étrangère entre en garnison si les
   murs peuvent la tenir entière.
9. L'IA ne poursuit plus une armée ennemie réfugiée sur des terres où elle ne passerait pas.
10. Une route du graphe des colonies qui traverse (ligne droite échantillonnée) une province
    fermée est fermée ; celle qui traverse des terres ouvertes par tempérament coûte
    `trespass_route_factor` fois plus (`data/ai/grid.json`, 2 ; 1 par défaut).
11. L'IA demande le droit de passage (accès militaire, réciproque s'il manque) aux royaumes IA
    sur les terres desquels ses armées se trouvent, s'ils l'accepteraient.

## Conséquences

Mesures détaillées : `docs/wip/eq5-ia-banqueroutes-intrusions.md` (mêmes graines qu'EQ4 ;
`century_probe` 464 tours, normal et difficile 10 graines, facile et très difficile 5).

| Mesure | Facile | Normal | Difficile | Très difficile |
|---|---|---|---|---|
| Banqueroutes / fac. / déc. (EQ4 → EQ5) | 0,42 → 0,08 | 0,17 → 0,04 | 0,19 → 0,03 | 0,09 → 0,01 |
| Pire faction sur une graine (/ déc.) | 31,4 → 1,1 | 6,7 → 1,1 | 7,8 → 2,0 | 5,4 → 0,3 |
| Saisons d'intrusion / siècle | 593 → 251 | 1312 → 474 | 2216 → 657 | 2472 → 745 |
| Casus belli d'intrusion / siècle | 48 → 17 | 101 → 43 | 177 → 54 | 193 → 73 |

- Guerre FR-EN, trêves, révoltes, boule de neige, survie des majeures en 1400 : dans les bandes
  d'EQ4 en moyenne (détail et écarts par graine dans la note de travail).
- `balance_probe` 16 × 200 : impôt Haut 30-32 % (< 40 %), banqueroutes 0,07-0,11, révoltes
  3,5-4,2 par partie (bas de la bande 4-10).
- Coût : le tour de l'IA est un peu plus long (table de retour au pays et échantillonnage des
  routes, mis en cache par tour ; atteignabilité par composante de la grille, sans A*).
