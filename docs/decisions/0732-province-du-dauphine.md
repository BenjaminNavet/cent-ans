# 0732 — Province du Dauphiné de Viennois, achat de 1349 et objectif français

Statut : accepté (23/10, lot DAUPHINE ; `out/juge/iter-02.md` C3, `out/juge/iter-02-histux.md` partie A constats 1 et 3).

## Contexte
Le Dauphiné (Grenoble, Vienne) était fondu dans `prov_provence` (« province composée », propriétaire `fac_naples`).
Conséquences : l'événement `evt_achat_dauphine` (1349) exigeait `controls fac_empire prov_provence` et ne se
déclenchait jamais ; l'objectif `obj_fr_dauphine` visait `prov_provence`, c'est-à-dire Marseille et le royaume de Naples.
Historiquement le Dauphiné de Viennois est un fief d'Empire tenu par Humbert II, qui le cède à Charles (V) en 1349 ; la
Provence est à Robert d'Anjou puis Jeanne Ire.

## Décision
1. **`prov_dauphine`** (Grenoble ; Vienne, Grande Chartreuse), propriétaire **`fac_dauphine`** (non jouable, suzerain
   `fac_empire`, souverain Humbert II, titre `tit_dauphine`). Humbert II passe de `fac_empire` à `fac_dauphine`.
   Population répartie 40 % / 60 % (Dauphiné / Provence), `base_supply` 2 100 / 3 150 (total 5 250 conservé).
   `prov_provence` ne garde que Aix et Les Baux ; sa description et ses fleuves sont corrigés ; la Grande Chartreuse
   passe au Dauphiné.
2. **Événement 1349** : conditions `controls fac_dauphine prov_dauphine` ; l'effet est un `transfer_title tit_dauphine`
   (prix 12 000 livres payés par la France) et non un `transfer_province` : la province est la capitale de
   `fac_dauphine`, qu'un transfert simple refuse (LR-17). La vente absorbe la faction vendeuse.
3. **Objectif** `obj_fr_dauphine` : `control_all [prov_dauphine]`.
4. **Carte composée** : voie la moins invasive = `data/map/compose/id_map.json` (`prov_dauphine` -> `prov_dauphine` au
   lieu de `prov_provence`) : l'ancienne province réelle `prov_dauphine` de `data/map/real/` (443 provinces) est conservée
   telle quelle, sans graines à régler. `selection.json` est mis à jour à la main de façon équivalente (la province est une
   capitale de faction, donc protégée de la fusion si `compose select` est relancé). `province_splits.json` n'est pas
   utilisé (il resterait l'outil pour couper une province dont l'ancienne géométrie n'existe pas).
5. Les `neighbors` de Beaucaire, Comtat-Venaissin, Provence et Savoie sont recalés sur la géométrie (test existant).
   Horizon (`alps`), biome de bataille (6) et décor (`montagne`) de `prov_dauphine` copient ceux de la Savoie.

## Procédure carte (rejouable)
Prérequis : `data/map/real/` (gitignoré) copié du checkout principal. Puis, depuis la racine :
`geo provinces`, `geo settlements`, `geo roads --computed`, `geo splat`, `geo rivers-render`, `geo hamlets`, `geo navgrid`,
`geo colormap`, `geo sea-lanes`, `compose authored-px`, `geo horizon`, `python -m cent_ans_tools.compose.cross_fixture`
(sans `geo compose`, `compose-relief`, `compose-layers` : ils écraseraient les retouches de côte). Aucun fichier de
`~/dev/cent-ans-raw` n'est lu ni écrit.

## Conséquences
- 216 provinces. L'achat se déclenche pour la France (printemps 1349) ; le joueur peut aussi prendre le Dauphiné par
  conquête, auquel cas l'événement ne se déclenche plus (condition `controls`).
- Données par faction à compléter si un outil l'exige (blasons : `fac_dauphine` porte le sien ; maison La Tour-du-Pin existante).
