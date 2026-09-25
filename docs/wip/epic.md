# Batailles épiques (session du 25/09)

Demande du joueur : rendre les batailles épiques. Carte de bataille réaliste et accidentée (collines,
rivières, ponts, villages), horizon peint ou lointain hors de la zone jouable (mer, montagnes), unités
massives portant chacune un étendard (une figurine porte-étendard), cris et fracas d'armes quand la
caméra s'approche d'une mêlée, et toute autre idée qui sert l'épique. Autonomie complète, **plafond de
20 $** pour les assets générés (consigné dans `docs/budget.md`, section « Batailles épiques »).

Réponses du joueur au lancement :
- échelle : **15 000 soldats et plus** au total ;
- générateur procédural enrichi **et** 3 cartes historiques faites à la main : **Crécy, Poitiers, Azincourt** ;
- horizon **hybride** : relief réel autour du site en maillage lointain + panorama peint (IA).

Worktree d'intégration : `../gp-epic-merge` (branche `integration/epic`), fusion ff-only dans main,
commits avec chemins explicites. Au plus 6 agents à la fois (machine partagée avec l'orchestrateur de
nuit : NV2, SG3, EQ1, PF1, DP2, AR1).

## Constat de départ (audit du 25/09)
- Champ 1200 × 800 m, grille de relief 10 m ; anneaux lointains décoratifs déjà présents (3 km, ±7 km à
  200 m) mais sans panorama ni relief réel ; caméra jusqu'à 900 m, plan lointain à 16 km.
- **Plafond `MAX_ON_FIELD = 20` régiments par camp** (`sim/reinforcements.rs:13`) : ≈ 4 800 soldats au
  plus sur le terrain. Banc T2 : 28 752 soldats à 30 i/s ; BV3 : 11 800 à 55 i/s avec imposteurs.
- Une seule rivière (18 m, 2 gués, eau profonde lente mais franchissable), **aucun pont**, routes
  purement décoratives, un seul village (6 à 11 bâtiments) ou une ferme.
- Étendard porté par une figurine ordinaire du rang (`bearer_rank`), pas de modèle dédié, visible à
  moins de 220 m ; pas de chute ni de prise d'étendard.
- Audio : une seule « nappe » de mêlée par type, au barycentre ; ≈ 60 clips de bataille (4 chocs
  d'épée, 7 râles, 3 cris de charge) : trop peu pour une foule.
- Aucune carte de bataille fixe ; « Crécy » n'existe que dans des illustrations.

## Lots

| Lot | Objet | ADR | Dépend de | État |
|---|---|---|---|---|
| EP1 | Échelle massive : champ plus grand selon l'effectif, plafond de régiments relevé, rendu 15 000+ (imposteurs très lointains, budget d'animation par distance), banc ≥ 40 i/s | 0031 | — | à lancer |
| EP2 | Horizon : relief réel (DEM) autour du lieu en anneau lointain, panoramas peints par région (mer, Alpes, Pyrénées, collines), silhouettes lointaines (clocher, château), brume de chaleur/fumée de camp | 0032 | — | à lancer |
| EP3 | Eau et chemins : plusieurs cours d'eau et ruisseaux, ponts de bois et de pierre (goulots), gués multiples, routes qui accélèrent la marche, IA qui tient ponts et gués | 0033 | — | à lancer |
| EP4 | Son de mêlée de proximité : émetteurs par front de mêlée, couches proche/moyen/lointain, grande banque CC0 (chocs, cris, râles, chevaux, ordres), foule qui monte avec l'effectif | — | — | à lancer |
| EP5 | Étendards : figurine porte-étendard dédiée (pose et clips), musiciens (tambours, trompettes), étendard qui tombe, relevé ou pris (moral, écran de fin) | 0034 | — | à lancer |
| EP6 | Villages et décor du champ : hameaux variés, moulin à vent/à eau, église et cimetière, manoir fortifié, vignes, vergers, meules, charrettes, camp et convoi derrière les lignes, pieux | — | EP3 | vague 2 |
| EP7 | Cartes historiques Crécy (26/08/1346), Poitiers (19/09/1356), Azincourt (25/10/1415) : relief réel, décor d'époque, déploiement historique, entrée depuis la campagne et le menu | 0035 | EP1-EP3, EP6 | vague 2 |
| EP8 | Mise en scène : heure du jour (aube, crépuscule), ombres de nuages, poussière des charges, fumées, oiseaux qui s'envolent, caméra cinématique au premier choc | — | EP2 | vague 2 |

## Budget (plafond 20 $)
Prévu : panoramas EP2 (≈ 12 images, ≈ 0,6 $), fonds de cartes historiques EP7 (≈ 0,3 $). Sons : banques
CC0 gratuites (Freesound, `tools/cent_ans_tools/freesound_search.py`). Réserve pour génération 3D
éventuelle (porte-étendard, moulin) si le kit Blender ne suffit pas.

## Reprise
1. `git worktree list` : worktrees des lots EP*, fichier wip `docs/wip/ep<N>-*.md` dans chacun.
2. Fusion : dans `../gp-epic-merge`, `git merge main` puis la branche du lot ; tests ; ff-only dans main.

## Décisions
- 25/09 : ADR 0031 à 0035 réservées pour ce chantier (0029-0030 laissées à la nuit : PF1 prévoit un ADR).
- Taille du champ : EP1 rend les dimensions du champ paramétriques (plus de constantes figées) ; EP3 et
  EP6 ne doivent jamais supposer 1200 × 800 et lisent les dimensions du champ.
