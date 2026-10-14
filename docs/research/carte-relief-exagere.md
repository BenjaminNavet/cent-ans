# Carte de campagne à relief exagéré (façon Total War) — rapport

Date : 2026-10-13. Demande du joueur : « la carte au niveau réaliste et à l'échelle réelle n'est pas
intéressante à jouer ; il faudrait une carte exagérée sur les reliefs, pas forcément très réaliste,
un peu comme la carte de l'Europe de Total War ». Rapport seul, aucun code modifié.

## 1. État actuel (mesuré)

| Élément | Valeur | Source |
|---|---|---|
| Emprise | 7 168 × 6 144 px × 719 m ≈ **5 150 × 4 420 km**, projection EPSG:3035 | `data/map/map.json` |
| Exagération verticale | `HEIGHT_SCALE` 0,006 unité/m × 719 m/unité = **×4,3** | `game/scripts/map/map_data.gd:14` |
| Rehaussement local | `h + 0,8 · clamp(h − flou σ 5 km, ±120 m)`, effacé au-dessus de 600-1 600 m | ADR 0019, `map.json.render_heightmap.boost` |
| Caméra de jeu | distance 180 → 1 250 unités (≈ 130 → 900 km) | `data/ui/campaign_map.json` |
| Mouvement | coût par case de 1,44 km : plaine 10, colline 15, forêt 18, marais 25, montagne 30 ; infranchissable si pente > 0,22 ; cols de `crossings.json` forcés franchissables | `data/movement/rules.json`, `tools/cent_ans_tools/geo/navgrid.py` |
| Terres de la grille | plaine/route 24 %, collines 17 %, forêt 41 %, marais 2 %, montagne 8 %, infranchissable 8 % (pente, grands fleuves, hors provinces) | mesure sur `navgrid.png` |

Ce que cela donne à l'écran : le Mont-Blanc (4 800 m) monte à 29 unités pour des Alpes larges
d'environ 250 unités ; la chaîne a un rapport hauteur/largeur de ~0,1, soit une **houle**, pas un
mur. Le Massif central (1 000-1 800 m, 6 à 11 unités sur ~400) disparaît à hauteur de jeu. Le
rehaussement local (σ 5 km, ±120 m) ajoute du **grain** (collines fines), pas des **masses** :
c'est exactement l'effet « carte IGN » déjà combattu par l'ADR 0168.

Côté règles, le relief est un **dégradé de coûts** (une montagne coûte 3 plaines) et non une
**structure** : on contourne ou on traverse lentement presque partout. Rien ne force une armée à
passer par un col, rien ne découpe l'Europe en théâtres.

L'ADR 0158 (carte généralisée) a déjà grossi les villes, fleuves, routes et arbres, mais a gardé
**« le relief reste vrai »**. C'est ce point qu'il faut maintenant renverser.

## 2. Ce que font les cartes Total War (Rome II, Medieval II, Warhammer)

Observations qualitatives (ordres de grandeur estimés à l'œil, pas mesurés dans les jeux) :

1. **Montagnes caricaturées** : exagération verticale de l'ordre de ×15 à ×40 par rapport à
   l'horizontal, chaînes **plus étroites** que dans la réalité, crêtes vives, roche et neige
   franches. Une chaîne se lit comme un mur à toutes les distances.
2. **Plaines aplaties** : le micro-relief est supprimé ; le contraste plaine/montagne fait tout.
3. **Cols et vallées sculptés** : quelques passages nets, creusés, souvent gardés par un fort ou
   une ville. Le relief crée des **goulets** et donc des batailles prévisibles et intéressantes.
4. **Montagnes infranchissables hors des cols** : c'est la règle de jeu qui donne son sens au
   relief (Pyrénées, Alpes, Carpates, Balkans deviennent des frontières naturelles).
5. **Distances compressées et théâtre principal agrandi** : la géographie est déformée
   (cartogramme) pour que chaque province compte et que les marches soient courtes.
6. **Lecture « maquette peinte »** : couleurs par grandes masses (plaine verte, piémont brun,
   roche grise, neige), pas de détail photographique.

## 3. Ce qu'il faudrait changer

### R0 — Planche d'essai (à faire d'abord, 0 $)
Générer une heightmap stylisée (R1) avec trois niveaux d'exagération (montagnes ×12, ×20, ×30)
et une planche de captures headless à hauteur de jeu sur quatre lieux : Alpes, Pyrénées, Massif
central, Beauce/Île-de-France. Le joueur choisit le niveau ; le reste en découle. Coût : une
demi-journée d'agent, 4 à 8 captures.

### R1 — Relief stylisé « caricaturé » (rendu)
Nouvelle commande `cent-ans geo stylize` qui produit `heightmap_render.png` (et les tuiles E0)
depuis le relief réel, réglages dans `map.json.render_heightmap.stylize` (données, schéma) :

- **Séparer les fréquences** : basse fréquence (massifs, ~20-40 km) gardée et amplifiée ; haute
  fréquence (< 5 km) supprimée en plaine, conservée sur les crêtes. Remplace le rehaussement
  local actuel (σ 5 km), qui fait l'inverse.
- **Courbe d'altitude non linéaire** : plaines < 300 m presque plates, collines modérées
  (×6-8), montagnes fortement amplifiées (×15-30 selon R0). Exagération appliquée à
  l'altitude **au-dessus du piémont**, pour ne pas soulever les plateaux entiers.
- **Chaînes resserrées** : érosion morphologique des piémonts (la crête garde sa ligne, ses
  flancs raccourcissent) → murs plus étroits, vallées et plaines plus larges entre eux.
- **Crêtes vives** : bruit « ridged » orienté selon la ligne de crête réelle, sur les seules
  zones de montagne.
- **Cols et vallées creusés** : entailles nettes le long des cols de `crossings.json` et des
  grandes vallées (Rhône, Pô, Garonne, Danube) ; les fleuves sont posés au fond (creusement le
  long de `rivers.geojson`) pour qu'aucun ne grimpe un versant.
- **Plateformes** sous chaque lieu habité (aplanissement local) pour que les maquettes de l'ADR
  0158 ne penchent pas sur une pente devenue raide.

Comme tout le rendu passe par `MapData` (surface unique), armées, villes, routes, fleuves,
végétation et sélection suivent sans code supplémentaire. À recalculer : `relief_occlusion.png`
(ADR 0168), colormap, biomes/roche/neige (la limite de neige doit suivre l'altitude **stylisée**).

### R2 — Le relief devient une règle (cœur Rust + `navgrid`)
- `navgrid` calculé sur le relief stylisé : **toute la masse montagneuse infranchissable**, sauf
  les cols et vallées nommés. Liste de cols historiques à compléter (Mont-Cenis, Grand-Saint-
  Bernard, Simplon, Saint-Gothard, Brenner, Somport, Roncevaux, Perthus, cols des Carpates et
  des Balkans…) dans `crossings.json`, avec sources.
- Collines et piémonts restent franchissables mais lents ; plaines plus rapides (contraste).
- Effets de jeu des cols : fermeture ou coût doublé en hiver (`winter_mountain_factor` existe),
  bonus défensif au défenseur d'un col, place forte de col qui bloque le passage (zone de
  contrôle déjà présente).
- Vérifications : connexité de toutes les colonies (`navgrid.check`), seed bots en passe longue
  (bandes d'équilibre : durée des conquêtes, IA qui ne trouve plus de chemin), sauvegardes (armée
  posée sur une case devenue infranchissable → recalage sur la case libre la plus proche).

### R3 — Lecture visuelle des massifs
Roche et neige par grandes masses sur les crêtes, ligne des arbres nette, ombrage de volume plus
fort sur les montagnes seulement (le relief étant réellement haut, plus besoin de normales
gonflées), brume de vallée. Ajustements de `campaign_lighting.json` et des biomes ; pas d'asset
nouveau a priori.

### R4 — Déformation horizontale (optionnel, plus tard)
Agrandir le théâtre principal (France, Angleterre, Bénélux) et comprimer la périphérie (Rus',
steppe, Maghreb), comme Total War. C'est le plus gros chantier : toutes les couches
géographiques (provinces, côtes, fleuves, routes, lieux, pyramide, biomes, champs) seraient
reprojetées, sauvegardes incompatibles, distances de règles à revoir. À n'envisager qu'après
R1-R2, si la carte paraît encore trop vaste.

## 4. Conséquences et risques

- **Nouvel ADR** : il remplace le point « le relief reste vrai » de l'ADR 0158 et le
  rehaussement local de l'ADR 0019.
- **Pyramide de relief (ADR 0036, 0203)** : les étages fins E1-E3 sont du relief réel ; superposés
  au relief stylisé, ils le contrediraient. Deux voies : styliser aussi la pyramide (même
  traitement, plus long à cuire) ou la limiter aux étages utiles depuis le plancher de caméra
  relevé de l'ADR 0158 (à mesurer en R0). Recommandation : limiter.
- **Batailles** : le relief des champs de bataille (ADR 0020) vient du site réel. Deux choix :
  le garder réel (cohérent à l'échelle d'une bataille) ou lui appliquer une exagération modérée
  (×1,5-2) pour des collines plus marquées. Recommandation : le garder réel au départ.
- **Chantier GR en cours** (champs, habitat selon relief et eau) : ses règles de pente et
  d'altitude devront lire le relief réel (règle agricole) et non le relief stylisé (rendu).
- **Performances** : relief plus haut = plus de silhouettes visibles et plus de nœuds CDLOD ;
  mesure à faire sur machine calme.
- **Coût** : 0 $ (calcul local). Effort indicatif : R0 ½ jour, R1 2-3 jours d'agents, R2 2 jours
  + passe longue seed bots, R3 1 jour.

## 5. Recommandation

Enchaîner **R0 → R1 → R2 → R3** ; reporter R4. R0 tranche le niveau d'exagération sur images avant
tout travail lourd. R2 est ce qui rend la carte « intéressante à jouer » : sans montagnes
infranchissables hors des cols, l'exagération reste un décor.

Questions au joueur :
1. Quel Total War sert de référence visuelle (Rome II, Medieval II, Warhammer) ?
2. Montagnes infranchissables hors des cols : oui ?
3. Relief des batailles : réel ou légèrement exagéré ?
