# 0334 — IA de campagne : héritier désigné et réconciliation avec l'Église

Statut : accepté (lot TW `ai-camp`, suite des lots m2a/m2b, ADR 0325-0327).

## Contexte
Les lots m2a/m2b ont ajouté des actions (`Order::DesignateHeir`, interdit, conversion, croisade papale) dont
seul le joueur se servait. Audit de l'existant avant ce lot :
- Croisade papale : la branche de priorité 500 de `diplomacy::ai::war_target` et la borne `ai_max_participants`
  existent (ADR 0327) ; rien à brancher.
- Conversion : elle est passive (le seigneur d'une autre foi fait progresser la province chaque saison). Le seul
  levier actif est le prédicateur, déjà dirigé par l'IA vers les provinces à convertir (`agents/ai.rs`). Il n'y a pas
  d'ordre « convertir » à émettre : rien à ajouter.
- Excommunication : l'IA excommuniée et riche (> 8 000 livres) donnait 2 000 livres, valeurs codées en dur,
  sans tenir compte de l'interdit.

## Décision
- **Héritier.** `dynasty::ai_designate_heir` (appelé au printemps par `ai::campaign::characters`) : héritier par
  défaut = `Faction.heir`, sinon celui de la loi de succession ; candidats = membres majeurs et libres de la maison
  du souverain. Si le meilleur total de compétences (commandement + gouvernement + cour) dépasse celui du défaut
  d'au moins `ai_heir_min_skill_gap` (`data/rules/dynasty.json`, 25), l'IA émet `DesignateHeir` (égalité : le plus
  âgé, puis l'identifiant). Il faut un trésor d'au moins deux fois `designate_heir_cost`. Une fois désigné, il
  devient le défaut : pas d'ordre répété ni d'oscillation. Le malus de prestige d'un choix hors-loi est accepté.
- **Réconciliation.** `data/rules/religion.json` `ai_reconcile` (`excommunication_donation` 2 000, `reserve` 6 000 :
  équivalent de l'ancien seuil 8 000). Une IA catholique excommuniée donne le plus grand de `excommunication_donation`
  et `interdict.lift_donation` ; sous interdit seul, elle donne `lift_donation` (qui lève l'interdit) ; dans les deux
  cas seulement si le trésor garde `reserve` après le don. Sans la section, l'IA ne fait rien (mécanique inerte).
- Aucune conversion ni croisade nouvelle : voir Contexte.

## Conséquences
- Mesure `campaign_probe` (120 tours × 2 graines) avant/après : voir `docs/wip/tw-ai-camp.md`.
- Le critère de compétences est volontairement simple ; il ignore l'âge, les traits et la légitimité.
