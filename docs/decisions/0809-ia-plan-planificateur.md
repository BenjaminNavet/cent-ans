# ADR 0809 — IA-PLAN-1 : planificateur d'IA de bataille par divisions

(Numéro attribué à la fusion.)

## Contexte

L'audit de la bataille (`docs/audits/2026-10-29-bataille.md`, M3 / R3) relève que l'IA de campagne en bataille rangée est
une accumulation d'exceptions (`ai/plan_field`, ≈ 440 lignes, 86 réglages, horloges absolues) : chaque régiment vise
l'ennemi le plus proche, sans objectif ni phase. Le banc d'équilibre (ADR 0800) permet maintenant de juger une refonte.
IA-PLAN-1 pose le squelette du planificateur et un premier plan, défensif, sans toucher au comportement par défaut.

## Décision

- Drapeau `data/rules/battle_ai.json` `planner: "legacy" | "plans"` (défaut `legacy`). `BattleSim::set_planner` le
  remplace pour une bataille (banc, tests). Avec `plans`, les camps couverts par un plan passent par le planificateur ; les
  autres (et les sièges) gardent `plan_field`.
- `core/crates/sim-battle/src/ai/planner/` :
  - `division.rs` : trois divisions (avant-garde, bataille, arrière-garde), attribuées une fois, au premier tour d'IA après
    le déploiement, par profondeur (parts dans `battle_plans.json`). Chaque division a une **cible** : l'ennemi le plus
    proche de l'un de ses régiments, gardé tant qu'il reste à portée (`target_reach_m`) et pas dépassé de plus de 40 m par
    un autre.
  - `plans.rs` : `Plan { sides, phases }`, `Phase { roles, enter, exit }`, conditions **en données**
    (`{metric, op, value}`, métriques `time`, `own_losses`, `enemy_losses`, `enemy_shooter_share`, `enemy_has_stakes`,
    `force_ratio`, `enemy_distance`). Une phase se quitte quand toutes ses conditions `exit` (non vides) tiennent et cède la
    place à la phase suivante, dans l'ordre de la liste, dont `enter` tient. Jamais de retour en arrière.
  - `roles.rs` : `hold`, `shoot`, `reserve`, un rôle par division et par phase.
  - `mod.rs` : mémoire du planificateur (divisions, phase) dans `BattleSim` (hors empreinte de rejeu), mesure des
    conditions, application du plan.
- Plan *Tenir et recevoir* (défenseur) : phase `receive` (arrière-garde en `reserve`), puis `commit_reserve` quand les
  pertes propres dépassent 25 % (l'arrière-garde rejoint la ligne).
- **Ce que le plan change dans `plan_field`** (qu'il appelle encore pour le placement : terrain, ligne, tireurs, cavalerie,
  réactions) : posture tenue (pas d'épuisement de la patience, seule la sortie de lassitude `sallies` la lève), réserve
  prélevée sur l'arrière-garde quand sa phase l'y met (règle historique des quatre régiments de ligne conservée).
- **Discipline de cible** : après `plan_field`, tout ordre d'attaque d'un régiment de division vers un autre ennemi que la
  cible de division est reporté sur celle-ci si elle est à moins de `retarget_reach_m` (140 m), sinon le régiment tient
  (un régiment en mêlée continue). Un régiment libre encore fixé sur un autre ennemi est tourné vers la cible si elle est
  à cette portée. Assertion de debug (`obeys_division_targets`) : aucun ordre d'attaque d'un régiment de division ne vise
  autre chose que la cible de sa division.
- `BattleEvent` reçoit un champ typé `kind: Option<BattleEventKind>` ; `RefusalLikely` est écrit une fois dans le journal
  quand, sur un champ de bataille sans engagement, le silence dure `refusal_warning_share` (0,25,
  `battle_decision.json`) du délai de refus (échelle de rythme comprise). Indépendant du planificateur : il sert aussi
  l'IA historique. La bataille elle-même est toujours refusée au délai d'EP9 (ADR 0056).

## Conséquences

- Par défaut rien ne change (`legacy`). Le banc accepte `--planner legacy|plans` et `--seeds N`, et ajoute
  `median_refusal_likely_s` aux lignes qui en ont un.
- IA-PLAN-2 ajoute les plans offensifs et remplace `plan_field` rôle par rôle ; les rôles `fix`, `wear`, `strike`,
  `screen` s'ajoutent à `roles.rs`, la charge groupée de cavalerie (ADR 0802, `ai/mass.rs`) étant l'action `strike`.
- Limite connue : en phase `commit_reserve` le rôle `hold` de l'arrière-garde n'a pas d'autre effet que de supprimer la
  réserve ; le rôle `shoot` est déclaratif (les tireurs tirent déjà par `plan_field`).
- Le plan défensif a été jugé au banc sur 20 graines (`docs/wip/ia-plan-1.md`) : 349 victoires du défenseur sur 480
  contre 328 pour *legacy*, pertes égales. Avec une cible « barycentre de la division » la discipline de cible coûtait
  33 victoires ; avec « l'ennemi le plus proche d'un régiment de la division » elle est gratuite. Crécy et Castillon
  se dégradent (point ouvert pour IA-PLAN-2).
