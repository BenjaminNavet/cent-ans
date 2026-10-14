# Éditeur de carte composée

Page web locale pour retoucher la carte composée (lot CC, CE) sans écrire de JSON.

## Lancement

```
uv run --project tools cent-ans compose editor [--port 8766]
```

Ouvrir ensuite http://127.0.0.1:8766/ (le serveur n'écoute que sur 127.0.0.1 ; il n'ouvre aucune fenêtre).
Prérequis : la carte a été compilée au moins une fois (`geo compose`, `geo compose-relief`) et
`data/map/real/` existe (créé par le compilateur).

## Fond et calques

Fond : relief ombré de `data/map/heightmap.png` (~2400 px, mis en cache hors dépôt dans
`~/dev/cent-ans-raw/compose/editor/`), côtes, fleuves, lieux (noms au zoom), cols. Cases à cocher à droite.

## Outils et raccourcis

| Touche | Outil |
|---|---|
| `V` | Crêtes : glisser un sommet ; double-clic sur un segment = ajouter un sommet ; double-clic sur un sommet = le supprimer ; panneau de droite : nom, type (massif/collines), hauteur, largeur, supprimer |
| `N` | Nouvelle crête : clics pour poser les sommets, double-clic ou `Entrée` pour finir |
| `C` | Cols : clic sur une crête = nouveau col ; glisser un col le déplace le long de sa crête |
| `P` | Épingles : clic source (un lieu ou un point), clic cible ; glisser la tête pour déplacer ; rayon dans le panneau |
| `L` | Lieux : glisser un lieu pour déplacer sa position composée |
| `B` | Pinceau de masques (forêt, roche, champs, marais) : `Maj` efface, `[` `]` taille |

Molette : zoom. Déplacer : glisser le fond, clic droit ou `Espace` + glisser. `0` : tout voir.
`Ctrl+Z` / `Ctrl+Maj+Z` : annuler / rétablir (80 pas, masques compris). `Ctrl+S` : enregistrer. `Suppr` : supprimer la sélection. `Échap` : désélectionner.

## Enregistrement

Rien n'est écrit tant qu'on n'appuie pas sur **Enregistrer**. Chaque couche modifiée est validée
contre son schéma (plus : identifiants uniques, sommets dans la carte, lieux connus) puis écrite de
façon atomique (fichier voisin + `rename`). Une couche refusée n'est pas écrite et l'erreur s'affiche.

| Couche | Fichier | Schéma |
|---|---|---|
| Crêtes, cols | `data/map/compose/ridges.json` | `compose_ridges` |
| Épingles | `data/map/compose/pins.json` | `compose_pins` |
| Lieux | `data/map/compose/place_nudges.json` | `compose_place_nudges` |
| Masques | `data/map/compose/masks/{forest,rock,fields,marsh}.png` | voir ci-dessous |

### Crêtes d'auteur
Toute crête modifiée ou créée dans l'éditeur reçoit `"authored": true` et devient la source : ni
`ridges_author` (passe de `compose-relief`) ni `geo compose` ne la recalculent. Une crête générée
supprimée est inscrite dans `deleted_ids` et ne revient pas ; `chains_seeded` indique que les chaînes
de `ridges_author.CHAINS` ont été recopiées (une seule fois : modifier `CHAINS` n'a plus d'effet,
éditer la carte dans l'éditeur).

### Lieux : `place_nudges.json`
`{"id": <lieu de selection.json>, "target_px": [x, y]}` : position COMPOSÉE (pixels du canevas). Le
`lonlat` réel du lieu ne change jamais. Lu par `compose/nudges.py` (`place_positions`), utilisé par
`compose-relief` (plateformes sous les lieux) ; les couches dérivées (C4) doivent passer par la même fonction.

### Épingles
Source = point réel (`lonlat`), cible = position composée. Elles modifient le cartogramme : après
enregistrement, **Recompiler** relance `geo compose --rebuild-warp` (automatique si `pins.json` est
plus récent que `warp.json`, ≈ 2 min de plus).

### Masques peints : format
PNG 8 bits à niveaux de gris, une case = **16 px composés** (≈ 11,5 km) ; taille = `ceil(largeur/16) x ceil(hauteur/16)`
(443 x 382 pour 7080 x 6104), origine en haut à gauche comme la carte. 0 = absent, 255 = présent
(valeurs intermédiaires = bords lissés du pinceau). Un masque jamais peint n'a pas de fichier (lire = noir).
Le serveur convertit en niveaux de gris les PNG RGB(A) envoyés par le navigateur. Consommés par
`compose/masks.py` (lot C4a) lors de `geo compose-layers` (étape `landcover`) : forêt ajoutée ; champs et marais retirent la forêt ;
roche = lande ; marais = canal R de `wetlands.png` ; champs = part cultivée du splat (détail : `docs/geo.md`).
Relancer `geo compose-layers --only landcover,colormap` après avoir peint.

## Planche et recompilation

- **Planche** : calcule le relief de la zone visible (≤ 2200 px de côté, quelques secondes ; la première fois
  charge le relief réel, ≈ 5 s) avec les retouches NON enregistrées de crêtes et de lieux, et l'affiche sur
  le fond (cadre rouge). Approximation : le bruit diffère légèrement de la compilation complète.
  Les épingles ne sont pas prises en compte (elles déforment toute la carte).
- **Recompiler la carte** : enregistrer d'abord ; lance `geo compose` puis `geo compose-relief` (≈ 3-5 min) en
  tâche de fond, journal affiché à droite ; le fond est rafraîchi à la fin. Ces commandes réécrivent les
  fichiers générés de `data/map/` (heightmap, tuiles, ...) : à relire avec `git status` avant de commiter.

## Sécurité
Les requêtes qui écrivent exigent l'en-tête `X-Cent-Ans-Editor` (une page tierce ouverte dans le
navigateur ne peut pas écrire) ; écoute limitée à 127.0.0.1.

## Code
`tools/cent_ans_tools/compose/editor.py` (serveur et logique), `editor_static/index.html` (page),
`compose/nudges.py`, tests `tools/tests/test_compose_editor.py`.
