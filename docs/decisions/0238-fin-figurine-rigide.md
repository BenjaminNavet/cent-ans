# 0238 — Fin de la figurine rigide des soldats de bataille

## Contexte
Deux voies coexistaient pour dessiner un soldat : la figurine skinnée (`BattleSkinned`, manifestes V2/FG/GA3) et la figurine rigide (lots V4/B1/B4 : 27 glb `assets/models/battle/`, figurines procédurales de `BattleMeshes`, shader `battle_soldier.gdshader` de 787 lignes animant jambes, bras, cheval, arc). Vérification (lot SC bt3) : tous les types d'unité de `data/unit_types` déclarent une figurine présente dans le manifeste skinné fusionné ; la voie rigide n'était plus atteinte pour infantry/archer/cavalry que par les interrupteurs de comparaison `--rigid-figures` / `--legacy-figures` ou si le manifeste manquait. Les engins de siège (`render` = `siege`) n'ont en revanche aucune figurine skinnée : ils restent rigides.

## Décision
- Supprimés : les 27 glb rigides et `figures.json`, leurs générateurs Blender (`tools/blender/battle_figures.py`, `preview_figures.py`), les figurines procédurales de soldats de `BattleMeshes` (infanterie, archers, cavalerie, cheval, cavalier, écu, lance, chargeur glb), `BattleSkinned.rigid_variant` et `enabled()`, les drapeaux `--rigid-figures` / `--legacy-figures`, le test `b1_mesh_stats`.
- Conservés : dans `BattleMeshes`, les engins (mangonneau, trébuchet, bombarde, servants de repli), arbres, rochers, hampe, drapeau ; `battle_soldier.gdshader` réduit aux engins (verge animée, chute des cadavres, lisibilité à distance, matières).
- Comportement changé : un soldat non-engin sans figurine skinnée n'a plus de repli rigide ; `BattleMeshes.soldier_level` journalise une erreur et renvoie un maillage vide (cas non atteignable avec les données actuelles).

## Conséquences
- Plus de comparaison avant/après avec les figurines rigides (l'historique git les conserve).
- Toute nouvelle unité non-engin doit avoir une figurine déclarée dans un manifeste skinné.
