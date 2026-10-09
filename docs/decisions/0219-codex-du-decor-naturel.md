# 0219 — Codex du décor naturel et bulle de survol différée

- Statut : accepté (2026-10-09, chantier NA)

## Contexte

Le décor naturel (23 essences d'arbres de campagne, 7 de bataille, 32 animaux, 8 oiseaux, 6 types
de rochers) n'avait aucune fiche au codex (seuls le saule, le genévrier et le loup existaient, par
la médecine ou l'histoire). Le joueur veut s'informer en survolant un élément, sans être harcelé de
bulles. Tout ce décor est dessiné en MultiMesh, sans nœud ni collision par instance.

## Décision

- Nouvelle famille « Nature » du codex (catégories `arbre`, `animal`, `oiseau`, `roche`).
- Champ `decor` des fiches : clés `tree:<id>`, `battle_tree:<id>`, `fauna:<id>`, `bird:<id>`,
  `rock:<id>` vers les ids des données de rendu ; une clé n'appartient qu'à une fiche (validé par
  `cent-ans validate`). Les variantes proches partagent une fiche (bovins, cerf et biche…). Champ
  `latin` pour la bulle.
- Survol : pas de collision ni de nœud. Après une immobilité de la souris (délai réglable, 1,5 s par
  défaut, 0 = désactivé), un pick CPU ponctuel lit les données déjà en mémoire (instances des
  MultiMesh, centres de troupeaux), au point du sol sous le curseur. Armées et villes restent
  prioritaires. La bulle est une bulle codex ordinaire (`CodexBubbles.open`) : clic → fiche.
- Les oiseaux (mobiles, animés dans le shader) et l'herbe/fleurs de bataille (placées dans le
  shader) ne sont pas survolables ; les fiches d'oiseaux existent quand même.
- Aucune règle de jeu : tout reste dans `game/` et `data/`.

## Conséquences

- Coût nul hors survol : le pick ne tourne qu'une fois par immobilité.
- Le pick des troupeaux est approximatif (mouvement dans le shader) : on vise le troupeau, pas la bête.
- Ajouter une espèce au rendu sans fiche reste valide ; la bulle ne s'affiche simplement pas.
