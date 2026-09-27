# 0093 — Rester sur Godot (pas de migration vers Unreal)

Date : 2026-09-27. Statut : accepté.

## Contexte
Le joueur s'est demandé si migrer vers un autre moteur rendrait le jeu plus beau. Un prototype
(`tools/proto_moteur/`, notes `docs/wip/proto-moteur.md`) a rendu la même petite section (terrain,
mur, grange, sapins, herbe, 12 fantassins, lumière du matin) avec les mêmes assets et la même caméra
sous Godot 4.7 (Forward+, SDFGI, SSIL, brouillard volumétrique, réglages au maximum) et sous
Unreal 5.7 (Lumen, Nanite, ombres virtuelles, nuages volumétriques), piloté par agent via le
protocole Python Remote Execution (celui du MCP `runreal/unreal-mcp`).

## Décision
Le jeu reste sur Godot. Pas de migration vers Unreal ni Unity.

## Raisons
- À assets identiques, l'écart visuel est faible. Unreal gagne sur le ciel (nuages
  volumétriques natifs), la lumière d'ambiance et les ombres lointaines ; aucun des deux moteurs
  n'améliore les figurines. Le goulot est la qualité des assets.
- Coût de migration : réécriture de tout `game/` (batailles, figurines, relief en streaming,
  végétation, UI, shaders). Seul `core/` (Rust) serait portable.
- Coût de développement par agent estimé 1,5 à 3 fois plus élevé sous Unreal pour le rendu et
  l'UI : assets binaires (`.uasset`) à interroger via un éditeur ouvert plutôt qu'à lire en texte,
  API Python vaste et tâtonnante, pièges macOS (multicast, fenêtre masquée qui ne dessine pas),
  fusions git impossibles sur un même asset, ce qui casse les vagues d'agents en parallèle.
- Unreal reste la plateforme la moins soignée sur Mac, poste de développement principal.

## Conséquences
- Les gains visibles d'Unreal se cherchent dans Godot : ciel nuageux volumétrique (shader de ciel
  ou `FogVolume`), étalonnage par heure du jour, puis figurines plus fines.
- Le projet Unreal de test (`~/dev/cent_ans_ue_proto`, hors dépôt) n'est plus nécessaire.
