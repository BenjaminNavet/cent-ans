# 0353 — Relief exagéré façon Total War et massifs infranchissables hors des cols

Statut : accepté (13/10, nuit RE). Renverse le point « le relief reste vrai » de l'ADR 0158 et
l'écrasement des montagnes SZ1 (`mountain_ratio`, `mountain_knee_m`).

## Contexte
Le joueur juge la carte à l'échelle réelle « pas intéressante à jouer » et veut des reliefs
exagérés, façon Rome II / Warhammer, avec des montagnes infranchissables hors des cols ; le relief
des batailles reste réel. Mesure (rapport `docs/research/carte-relief-exagere.md`, recherches
`docs/research/relief-exagere/`) : l'écrasement SZ1 ramenait les Alpes à ≈ ×1,6 par rapport aux
plaines, une houle plutôt qu'un mur. Côté règles, une montagne coûtait seulement trois cases de
plaine et rien ne forçait le passage par un col.

## Décision
**Rendu (hauteur affichée, une formule pour GPU, GDScript et Rust)**
- `y = s·(h − K·max(h − base, 0) + g·(1 − K)·max(h − fond, 0) + c·D)`, `K = c·k`.
- `k` peut devenir **négatif**. `k < 0` amplifie le relief au-dessus de la base :
  `mountain_lift` 0,5, de 350 m à 1 400 m. `k > 0` aplatit le micro-relief des plaines
  intérieures (`plain_flatten` 0,6, jusqu'à 120-350 m).
- **D**, 4ᵉ canal du champ (`RGBAF`), est un **relevé de massif lisse** :
  - formule : `massif_lift · max(milieu lissé − base, 0)` ;
  - `massif_lift` 4, lissage `massif_smooth_radius` 2 ;
  - effet : des massifs en bloc, sans les aiguilles qu'un `k` seul produisait de près.
- **Côtes** : relevé et aplatissement s'effacent à moins de `massif_coast_fade_cells` d'une
  côte ; les falaises gardent leur hauteur.
- **Boîtes englobantes** : le terme additif `c·D_max` est ajouté par `relief-lod` (`View.lift`),
  le quadtree, les rubans fins, les tronçons et les villes.
- **Batailles** : intouchées, elles lisent l'altitude vraie.
- Les règles de pente et d'altitude (champs GR, habitat, navgrid) lisent toujours le relief réel.

**Règle (grille de navigation, pipeline)**
- `rules.json.massif_impassable` (schéma) : une case est une **masse** si son altitude lissée
  (± 2 cases) atteint 1 500 m et dépasse de 700 m le bas pays régional (minimum sur ± 20 cases,
  lissé).
  - Les masses sont infranchissables, sauf les cols de `crossings.json`, taillés comme une pente
    raide.
  - Les vallées intérieures (Valais, Aoste, Inn) et les hauts plateaux (Anatolie, Meseta)
    restent franchissables.
  - Alpes, Pyrénées centrales et Caucase deviennent des murs ; les Carpates et le Massif central
    restent lents mais franchissables.
- **Cols** : 24 en tout. Douze s'ajoutent aux douze existants :
  - Alpes : Splügen, Septimer, Resia, Lautaret, Larche, Arlberg, Lukmanier, Grimsel ;
  - Caucase : Darial ;
  - Balkans : Chipka, Porte de Trajan ;
  - Taurus : Portes de Cilicie.
  Ils sont sourcés et touchés par le rabais de col et le malus d'hiver déjà en place (RG9,
  ADR 0343).
- **Colonies enclavées** : six colonies (Altdorf, Rila, quatre au Caucase) reçoivent un **sentier
  de montagne** au coût montagne. C'est le plus court chemin vers le reste du pays, signalé dans
  le rapport du pipeline.
- **Sauvegardes** : au chargement, une armée en campagne posée sur une case devenue infranchissable
  est recalée sur la case libre la plus proche (`march::unstick_field_armies`).

## Conséquences
- 89,3 % des terres sont franchissables, environ 3 points de moins qu'avant.
- Les marches transalpines et transpyrénéennes passent par les cols.
- L'IA et les seed bots s'en ressentent : passe longue lancée, constats à trier.
- Bonus défensif au défenseur d'un col : non fait. Il faudra un modificateur de bataille lu au
  point de rencontre.
- Le relief affiché et la règle sont calculés séparément mais sur les mêmes grandeurs (altitude
  au-dessus de la base). Un massif dessiné haut est un mur.
- Réglages à revoir à l'œil par le joueur : `relief_exaggeration.tres` pour le rendu,
  `massif_impassable` pour la règle.
