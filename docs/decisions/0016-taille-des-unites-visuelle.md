# ADR 0016 — Taille des unités : multiplicateur visuel de figurines

Date : 2026-09-25. Statut : accepté. Lot BV1 (« bataille vivante »), demande du joueur.

## Contexte

Le joueur veut plus de soldats par unité, comme le réglage « taille des unités » de Total War :
Petite / Normale / Grande / Ultra (×0,5 / ×1 / ×1,5 / ×2,5). Aujourd'hui, une figurine = un homme
simulé. Les effectifs viennent de la campagne (`data/units/`, pertes et rançons rendues à la carte).

## Options

- **A. Vrai changement d'effectifs dans `core/`** : le setup de bataille multiplie `soldiers` et
  `max_soldiers`, et divise les pertes au retour. Le tir (`shots × précision`) et la mêlée
  (`fighting_soldiers`) sont linéaires en effectifs, mais le front (`ranks_files`), donc l'emprise,
  les contacts, les débordements, la poursuite, les murailles, la fatigue et la déroute (pertes
  relatives) changent. Toutes les constantes d'IA réglées par B4, B6 et B8 (duel d'archerie, laisse
  de poursuite, attente de l'attaquant) et les empreintes des tests de régression seraient à
  refaire, pour chaque taille. Les batailles résolues automatiquement divergeraient des batailles
  jouées. Coût CPU de la simulation ×2,5 en Ultra.
- **B. Multiplicateur visuel** : la simulation garde ses hommes ; le pont dessine
  `round(soldats × k)` figurines, qui remplissent le rectangle simulé du régiment. Aucun effet sur
  l'équilibre, les sauvegardes, l'IA, l'auto-résolution ni les tests.
- **C. Mixte** : effectifs réels et règles normalisées par la taille. Le plus fidèle, mais c'est
  l'option A plus une couche de normalisation, et le même travail de réglage.

## Décision

**Option B.** `BattleSim.set_figure_scale(k)` (pont) ; `Unit::figure_positions(k)` et
`Unit::figure_count(k)` (`core/crates/sim-battle/src/unit.rs`) placent les figurines dans l'emprise
simulée (`extent()`) :
- lignes et colonnes gagnent des rangs autant que des files (√k de chaque côté), avec une distance
  minimale entre rangs (0,8 m à pied, 2,7 m à cheval, une longueur de cheval) ;
- carré et coin gardent leur forme, calculée pour `m` figurines, puis ramenée dans l'emprise ;
- à k = 1, le résultat est exactement `soldier_positions()` (aucun changement pour la taille
  Normale).
Les morts suivent : une figurine tombe quand `round(soldats × k)` baisse. Les volées (BV1) tirent
`k` fois plus de traits, et les touches donnent `k` fois plus de gerbes de sang.

Le réglage est `battle/unit_size` dans `Settings` (menu Réglages, onglet Partie) et s'applique à la
bataille suivante.

## Conséquences

- La densité, pas l'emprise, change : en Ultra, l'infanterie est serrée à environ 0,7 m. Ce serait
  faux pour une ligne réelle, mais c'est l'effet recherché : une masse compacte. En Petite, les
  rangs sont clairsemés.
- La fiche d'unité et le HUD affichent toujours les effectifs simulés, pas le nombre de figurines.
  Le joueur voit donc « 120 hommes » sur 300 figurines en Ultra : c'est assumé, comme la
  convention de Total War qui ne change pas les effectifs de campagne.
- Le coût est au rendu : figurines ×k (MultiMesh, LOD, ombres) et `get_soldier_buffer` ×k côté
  pont. Il est mesuré dans `docs/wip/bv1-bataille-vivante.md`. Des imposteurs au-delà de 300 m
  (backlog) restent la piste si l'Ultra est trop lourd sur les grosses batailles.
- Si un jour il faut de vrais effectifs, l'option C restera possible : aucun code de règle ne
  dépend du multiplicateur visuel.
