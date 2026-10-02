# 0161 — Habillage généralisé : forêts en volume et eaux lisibles à hauteur de jeu

Date : 2026-10-02. Statut : accepté. Prolonge l'ADR 0158 (carte généralisée).

## Contexte
Le joueur demande « plus de forêts, lacs, champs etc. pour habiller la carte de campagne ».
Mesures et lecture du code (inventaires du 02/10, `docs/wip/hc-habillage-carte.md`) :
- Les données sont déjà boisées : 39,7 % des terres dans `splat.png` (canal B). Le manque est de
  rendu : depuis l'ADR 0138 les arbres sont à l'échelle 1:1 (14-27 m) et coupés au-delà de la
  distance de rig 30 ; à hauteur de jeu (22-400) la forêt n'est qu'une teinte sombre et plate.
- Bosquets, ripisylves, vergers et arbres de haie existent dans le semis par essences, mais avec
  la même portée de 30 : invisibles en jeu.
- Lacs : 762 polygones, eau peinte très sombre (`lake_color`), proche de la teinte des forêts.
  Étangs et marais (`wetlands.json`, 32 zones) : teinte moyenne au dézoom, jamais de l'eau.
- Le masque de forêt et les zones humides alimentent la grille de déplacement (ADR 0045) et le
  couvert d'embuscade : y ajouter des surfaces change les règles.

## Décision
1. **Arbres généralisés.** Sous `map.tree_style = generalised` (défaut, `--tree-style=real` pour
   l'ancien rendu), les arbres ont une taille monde **constante et grossie**, indépendante de la
   distance de caméra (principe de l'ADR 0158), et sont dessinés jusqu'à la vue stratégique. Un
   arbre représente un bois ; les positions des massifs restent vraies. Les calques 1:1 (détail de
   forêt, cartes proches) sont éteints dans ce style.
2. **Plus de bois par le rendu, pas par les règles.** Bosquets, ripisylves, vergers, arbres de
   haie et garrigue du semis par essences deviennent visibles à hauteur de jeu. `splat.png`,
   `navgrid.png` et le couvert ne changent pas dans ce chantier.
3. **Eaux lisibles.** Lacs éclaircis et distincts des forêts, seuil de surface des lacs abaissé
   (rendu seul), étangs et mares des zones humides rendus comme des nappes d'eau généralisées qui
   restent lisibles au dézoom au lieu de se fondre en teinte.
4. **Champs** : le réglage d'échelle du parcellaire appartient à GC5. HC n'ajoute de variété
   (vignes, vergers, landes) qu'après la fusion de GC.
5. Toute extension des données qui touche les règles (nouveaux massifs nommés, nouvelles zones
   humides) fera l'objet d'un lot séparé avec recuisson complète et contrôle d'équilibrage.

## Conséquences
- La vue rapprochée perd les forêts 1:1 (cohérent avec l'ADR 0158, plancher de caméra relevé).
- Coût de rendu à mesurer (instances visibles à rig 300-700) ; paliers par distance de tuile.
- Les emprises d'exclusion (maquettes de ville, fleuves, lacs, routes) doivent tenir compte de la
  taille grossie des arbres.
- Réglages dans `resources/map_prop_scale.tres` et `data/ui/campaign_map.json`, pas en dur.
