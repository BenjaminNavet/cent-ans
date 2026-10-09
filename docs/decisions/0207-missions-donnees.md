# 0207 — Un seul moteur de missions, piloté par `data/missions.json`

Statut : accepté (lot SC CC13).

## Contexte
Les missions de campagne (ADR 0127) avaient une enum `MissionKind` à six
variantes et, dans `sim-campaign/src/missions.rs`, un `match` par variante à
quatre endroits : génération des cibles, verdict, libellé de progression,
compteurs. Ajouter une sorte de mission demandait de toucher le code.

## Décision
- `MissionKind` disparaît. Un gabarit de `data/missions.json` se décrit par
  trois axes : `target` (où le générateur cherche : `at_war`, `can_recruit`,
  `treaty_partner`, `enemy_neighbour`, `threatened_own`, `buildable`),
  `goal` (`control`, `hold`, `build`, `treaty`, `count`) et, pour `count`,
  `counter` (`battle_won`, `units_recruited`).
- Le moteur n'a plus qu'une mesure `(fait, demandé)` par `goal` : la mission
  réussit quand `fait >= demandé`, échoue à l'échéance (ou, pour `hold`, dès
  que la place est perdue). Les libellés de progression (`steps` ou
  `progress`) viennent des données, plus du code.
- La vue `get_missions` garde ses clés ; `kind` y porte désormais l'id du
  gabarit (l'interface ne l'utilise que comme texte). Pont et GDScript
  inchangés.
- Une mission active ne peut pas partager son gabarit avec une autre
  (auparavant : sa sorte) ; avec un gabarit par sorte c'est identique.

## Changements de mécanique
- Libellé de progression d'une prise de province introuvable : « personne »
  au lieu de « introuvable » ; « 1/1 victoire(s) » au lieu du pluriel
  calculé. Cosmétique.
- Format des sauvegardes : `Mission.kind` remplacé par `goal` + `counter`
  (sauvegardes cassées admises par le mandat SC).

## Simplifications évaluées et non appliquées
- Acomptes de rançon (`ransom.rs`, ordres, pont H5/H6, interface),
  ordres de chevalerie (`chivalry.rs`, 53 occurrences, IA, pont, événements)
  et hérésie → population (`religion.rs`, `agents.rs`, diplomatie) sont des
  mécaniques visibles du joueur, touchant IA, pont et interface. Les retirer
  appauvrirait le jeu sans économie nette de code dans ce lot ; ils restent
  à décider séparément (lot propre, avec arbitrage du joueur).

## Conséquences
- Une nouvelle mission combinant les axes existants s'écrit en JSON seul.
  Un nouvel axe (autre cible, autre but, autre compteur) ajoute une variante
  et une branche, mais une seule par axe.
