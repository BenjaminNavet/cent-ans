# VH — Villes historiques et peuplement à l'échelle du zoom géographique

Chantier **suivant** ZG (ADR 0036, `docs/wip/zg-zoom-geographique.md`). Demandé par le joueur le
2026-09-25 : « une fois [ZG] terminé, revoir les forêts, villes etc. qui peuplent la carte à cette
nouvelle échelle ; créer des représentations réalistes des villes de l'époque (Paris, Londres,
Orléans…) en profitant du fait que la carte soit plus précise ».

ADR réservé : **0037** (à rédiger au lancement de VH0, quand ZG2/ZG4 auront fixé l'API du relief).

## Pourquoi un chantier à part

Tout ce qui peuple la carte a été calibré pour 719 m/px :

| Élément | Aujourd'hui | Problème au zoom ZG (≈ 1 km puis ≈ 200 m) |
|---|---|---|
| Forêts | 55 ellipses nommées (`historical_forests.json`) + KK10 dans `splat.png`, `forest_kind.png` | lisières en ellipses et en pixels de 719 m, pas de clairières ni d'essarts, pas d'arbres individuels |
| Hameaux | 2 999 points (`hamlets.json`), densité « décorative » | la France de 1328 compte ~24 000 paroisses (État des paroisses et des feux) ; sites sans lien avec le relief (éperon, fond de vallée, source) |
| Villes emblématiques | 7 maquettes L1/L2 (`data/landmarks/*.json`) sous **loupe radiale** ×3,5 (ADR 0015) ; Paris = 13 rues, 22 monuments | la loupe déforme : sur un relief à 3 m, la Seine de la maquette ne tomberait plus dans la vraie vallée. Tissu urbain trop schématique pour une caméra à 200 m |
| Orléans | pas de maquette | ville-clé (siège de 1428-1429, pont et Tourelles), déjà zone de détail E5-E7 dans ZG |

## Partage avec ZG (à ne pas refaire)

- **ZG5** : routes, colonies, hameaux et ponts *recollés* au sol fin ; parcellaire procédural
  dans le shader. VH **ne refait pas** le recollage ; il change *quels* hameaux et forêts existent
  et où, puis réutilise le recollage ZG5.
- **ZG6** : villes *ordinaires* (569 colonies) à l'échelle réelle, faubourgs, finage. VH **ne
  refait pas** ZG6. Il prend les **villes emblématiques**, que l'ADR 0036 laisse volontairement
  sur leurs maquettes L1/L2.
- **ZG3** : zones de détail E5-E7. VH vérifie qu'Orléans, Paris et Londres y ont bien l'emprise
  intra-muros, les faubourgs et le finage proche (demi-côté ≥ 4 km pour Paris).

**Point à signaler à la session ZG** : l'ADR 0036 dit que les villes emblématiques gardent leurs
maquettes L1/L2. Il faudra un addendum : elles gardent la loupe en vue stratégique et passent en
1:1 au zoom rapproché (lot VH4).

## Lots

| Lot | Contenu | Dépend de |
|---|---|---|
| VH0 | Squelette : ADR 0037, schémas (`landmark` v2, `forest_mask`, `villages`), stubs, wip | ZG2 et ZG4 fusionnés (API `surface_height_at`, `vertical_scale()`, étages) |
| VH1 | **Forêts à l'échelle** : masque forestier 1340 à ~25 m (étages E3-E4) produit par `geo forests`. Base : couverture arborée actuelle (ESA WorldCover 10 m, déjà utilisée par ZG1), **corrigée** avec les forêts anciennes (Cassini et carte d'État-Major vectorisées, INRAE/EHESS, licence ouverte) et en retirant les plantations modernes (pins des Landes, XIXᵉ s. ; Sologne ; enrésinements), les ellipses R1 servant d'arbitre. Lisières, clairières, essarts autour des hameaux. Rendu : arbres instanciés (MultiMesh par tuile du quadtree, 3-4 essences selon `forest_kind`), imposteurs au loin | VH0, ZG1 |
| VH2 | **Villages et hameaux réels** : sites tirés de sources historiques (France : « Des villages de Cassini aux communes d'aujourd'hui », EHESS, ~37 000 lieux ; Angleterre : Open Domesday, CC BY-SA ; Flandre, Brabant : paroisses de l'OSM filtrées). Taille estimée depuis l'État des paroisses et des feux de 1328 (par bailliage) et le Domesday ; église, nombre de feux, type d'habitat (groupé au nord et à l'est, dispersé dans le bocage à l'ouest). Le parcellaire ZG5 se centre sur ces villages. Remplace `hamlets.json` (rendu seulement ; `settlements` du cœur intacts) | VH0, ZG5 |
| VH3 | **Recette des villes ordinaires** après ZG6 : cohérence avec VH1-VH2 (finage, lisières, faubourgs), densités calées sur les feux de 1328. Correctifs seulement, pas de refonte | ZG6, VH1, VH2 |
| VH4 | **Villes emblématiques 1:1** : format `landmark` v2 géoréférencé (EPSG:3035, plus d'ancre locale + loupe). Réseau de rues complet, îlots découpés en **parcelles médiévales en lanières** (façade de 5 à 8 m, profondeur 20 à 40 m), maisons du kit BR1 (pignon sur rue, colombage, encorbellement ; tuile, ardoise ou chaume selon le quartier), jardins et cours d'îlot, quais, moulins sur pont, ports. Monuments à l'échelle vraie avec gabarits détaillés. Rendu : HLOD par îlot (MultiMesh d'archétypes près, îlot fusionné au milieu, maquette L2 sous loupe en vue stratégique avec fondu entre les deux) | VH0, ZG4 |
| VH5 | **Paris vers 1340** (prioritaire) : enceinte de Philippe Auguste sur les deux rives ; enceinte de Charles V en variante datée (1356-1383) ; Cité (Palais, Sainte-Chapelle, Notre-Dame, Hôtel-Dieu), Grand-Pont et Petit-Pont habités, Châtelets, Louvre de Philippe Auguste (puis de Charles V, 1364-1380), Temple, abbayes de Saint-Germain-des-Prés et de Sainte-Geneviève, Saint-Victor, Halles, place de Grève, Pré-aux-Clercs, faubourgs. Sources : ALPAGE (parcellaire Vasserot vectorisé, licence ouverte), plan de Bâle, Atlas de Paris au Moyen Âge (faits seulement, pas de copie) | VH4 |
| VH6 | **Londres vers 1340** : mur romain et médiéval, Tour de Londres, Old St Paul's (flèche ≈ 150 m), London Bridge (maisons, chapelle Saint-Thomas, pont-levis), Guildhall, Cheapside, Southwark, Westminster (abbaye, Hall) relié par le Strand. Sources : Agas map (MoEML), Historic Towns Atlas (faits) | VH4 |
| VH7 | **Orléans vers 1340-1429** (nouvelle ville) : première enceinte et agrandissement de 1345-1350, pont sur la Loire avec les Tourelles et le boulevard, Châtelet, Sainte-Croix, faubourgs et églises hors les murs détruits en 1428. Même plan que le siège de 1429 (ADR 0026) | VH4 |
| VH8 | Migration des 5 autres maquettes (Rouen, Bordeaux, Avignon, Calais, Bruges) au format v2, recette visuelle à 1 km et à 200 m, performance (≥ 60 i/s au-dessus de Paris), captures comparées aux plans anciens, crédits et licences | VH5-VH7 |

Vagues (≤ 6 agents) : VH0 ; puis VH1 + VH2 + VH4 ; puis VH3 + VH5 + VH6 + VH7 ; puis VH8.

## Déjà faisable sans attendre ZG

- **Dossier de sources** (`docs/research/vh-sources.md`) : pour chaque source, emprise, licence,
  format, URL de téléchargement et usage prévu ; faits datés par ville (enceintes, ponts,
  monuments existants en 1340 ou non). Ce dossier ne touche ni au code ni aux données de jeu.

## État

- [x] Plan (ce fichier), 2026-09-25
- [ ] Dossier de sources (agent de recherche, lancé le 2026-09-25)
- [ ] Attente : fusion de ZG2 et ZG4 (API du relief), puis VH0

## Prochaine étape

Quand `docs/wip/zg-zoom-geographique.md` indique que ZG4 est fusionné : rédiger l'ADR 0037 et
lancer VH0. Si ZG6 n'est pas encore lancé, proposer à la session ZG l'addendum sur les villes
emblématiques (loupe en vue lointaine, 1:1 de près).
