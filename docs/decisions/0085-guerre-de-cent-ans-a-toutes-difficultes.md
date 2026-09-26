# ADR 0085 — La guerre de Cent Ans à tous les niveaux de difficulté (lot EQ6)

Date : 2026-09-26. Statut : accepté. Complète l'ADR 0054 (EQ3) et l'ADR 0037 (DF1).

## Contexte

Décision du joueur (2026-09-26) : la cible « France et Angleterre en guerre 55-75 % du siècle »
(EQ3, ADR 0054) vaut à tous les niveaux de difficulté (ADR 0037), pas seulement en Normale : la
difficulté doit rendre la guerre plus ou moins dure pour le joueur, pas décider si elle a lieu.

Mesure sur main (`century_probe`, 464 tours, mêmes graines qu'EQ4/EQ5 ; la France, jouée par
l'IA, porte les handicaps du joueur) : facile 48 % (1/5 graines dans la bande), normale 62 %
(8/10), difficile 56 % (5/10), très difficile 50 % (1/5, France éliminée en 1434 sur une
graine). Le tableau « EQ6 » de la sonde compte, à chaque tour de paix, ce qui empêche
l'Angleterre de déclarer la guerre :

- **Facile** : la raison d'attitude « Niveau de difficulté » (+10 envers le joueur) porte
  l'attitude anglaise au-dessus du seuil de la guerre de prétention (20) dans 60-70 % des tours
  de paix (graine 4 : paix de 1420 à 1449).
- **Difficile, très difficile** : l'Angleterre, plus riche, déclare sans cesse d'autres guerres
  (croisade contre Grenade, trône de Vérone hérité par mariage tous les trois ans) ; le repos de
  12 tours qui suit toute déclaration bloque la guerre de France 50-80 % des tours de paix.
  `war_target` classait les prétentions par rapport de forces : la plus petite couronne
  revendiquée passait avant la France.
- **Guerres sans fin** (facile, graine 1 : 1346-1406) : l'Angleterre battue (fatigue 100, score
  -82) n'a rien à céder ; la France gagne sans rien pouvoir prendre et refuse la paix blanche
  (article « Paix » à -88) ; le vainqueur ne se lasse jamais.
- **Très difficile** : la France, dominée 80-90 % du siècle et réduite à sa capitale, est
  « acculée » (lot G5) et achète la paix la saison même de chaque déclaration anglaise : jusqu'à
  21 guerres de 0 tour par siècle.

## Options

- **A. Retoucher les niveaux de difficulté** (moral, revenus de l'IA en très difficile) : essayé
  (moral 10 → 7, revenus 140 → 130) ; aucun effet net au-delà de l'écart entre graines, et cela
  rend le niveau plus facile pour le joueur. Rejeté.
- **B. Exempter la seule guerre de prétention principale** des effets de la difficulté et des
  distractions, et traiter les deux blocages de la paix (guerre sans fin, acculé qui capitule
  d'emblée). Retenu.

## Décision

Quatre règles, chacune derrière un réglage de `data/ai/diplomacy.json` (défaut sans le fichier :
comportement antérieur ; schéma `ai_diplomacy.schema.json`).

1. `war.main_claim_first` : la **prétention principale** d'une couronne est le trône revendiqué
   du plus grand royaume vivant (`diplomacy::main_claim` ; l'Angleterre : la France). Sa guerre
   passe avant toute autre prétention (priorité doublée dans `war_target`) et le repos qui suit
   une autre déclaration ne la retarde pas. Les autres garde-fous restent : trêve, fatigue,
   trésor, régence, souverain captif, front déjà trop lourd, rapport de forces, attitude.
2. `war.claim_war_ignores_difficulty` : pour cette seule guerre, ni l'attitude « Niveau de
   difficulté » ni le rapport de forces exigé (`ai_war_ratio_percent`) ne comptent. La difficulté
   pèse toujours sur les autres guerres contre le joueur, l'acceptation de ses offres,
   l'économie, l'agitation et le moral.
3. `negotiation.long_war_years` / `long_war_points_per_year` / `long_war_max_points` (8 / 8 /
   100) : au-delà de 8 ans d'une même guerre, chaque camp, vainqueur compris, ajoute la raison
   « Guerre interminable » (+8 par an, plafond 100) à l'évaluation d'un traité qui y met fin
   (`negotiation::context_reasons`). Une guerre que personne ne peut gagner finit en trêve
   (Brétigny 1360, Leulinghem 1389).
4. `peace.cornered_waits_for_defeat` : une couronne acculée, en guerre contre un prétendant à
   son trône, ne demande la paix avant `min_war_turns` qu'une fois battue dans cette guerre
   (score ≤ `SURRENDER_WAR_SCORE`, -25) (`negotiation::plan_peace`) : achetée, la paix ne
   ferait que ramener le prétendant après la trêve ; elle défend sa dernière terre. Contre tout
   autre ennemi elle traite aussitôt, comme avant (l'Écosse après Halidon Hill). Une première
   version sans cette restriction faisait disparaître l'Écosse (facile, 1396 et 1436) : entrée
   en guerre à l'appel de la France, acculée, elle ne traitait plus.

Aucune donnée de difficulté (`data/rules/difficulty.json`) n'est changée.

## Conséquences

Mesures détaillées et graine par graine : `docs/wip/eq6-guerre-toutes-difficultes.md`.

| Niveau | Guerre FR-EN avant | après | Trêves / siècle |
|---|---|---|---|
| Facile (5) | 48 % [32-70], 1/5 | **63 % [50-70], 4/5** | 6,4 → 11,6 |
| Normale (10) | 62 % [42-75], 8/10 | **69 % [60-72], 10/10** | 11,1 → 12,7 |
| Difficile (10) | 56 % [39-65], 5/10 | **60 % [45-72], 8/10** | 13,0 → 16,2 |
| Très difficile (5) | 50 % [41-60], 1/5 | **59 % [52-67], 4/5** | 15,4 → 15,6 |

- Plus longue guerre : 9-14 ans à tous les niveaux (72 ans en facile avant).
- Banqueroutes 0,02-0,06 / faction / décennie ; 1re faction en fin de partie ≤ 34 % ; les quatre
  majeures vivent en 1400 à tous les niveaux ; la France n'est plus éliminée en très difficile.
- `balance_probe` 16 × 200 (normale) : révoltes 3,9 → 6,5 par partie (dans 4-10) ; impôt Haut
  30-32 → 34-37 % (cible < 40, à surveiller).
- Empreintes et tests existants inchangés (les réglages sont neutres sans le fichier et les
  tests de déterminisme ne passent pas par ces branches) ; 4 tests dans
  `sim-campaign/tests/eq6_main_claim.rs`.
- Limites : la sonde joue la France par l'IA avec les handicaps du joueur ; en très difficile
  l'Angleterre domine encore le royaume 58-81 % du siècle. Les critères sont atteints de
  justesse en facile et très difficile (4/5) ; l'écart entre graines reste large.
