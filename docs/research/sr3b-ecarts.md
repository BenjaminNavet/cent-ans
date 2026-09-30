# SR3b — Écarts entre les figurines fines (FG2) et les références réalistes (SR3)

Sources : planche `sr3_references.jpg`, indices 4-6 de `sr3-references-realistes.md`, recettes
`battle_skinned_figures.FIGURES` (28 de bataille + 4 villageois) construites par
`battle_fine_figures.build_figure` (habits en coques du corps MakeHuman, casques/armes de
`battle_fine_gear` / `battle_fine_weapons`, FG0 de `battle_fine_equipment`).

Budget (bible DA § 6, ADR 0089) : plafond par figurine `TRI_CAP` 11 900 / 1 350 / 260 à pied,
`RIDER_CAP` 9 000 / 1 000 / 180 pour le cavalier (cheval en sus). `fit_budget` redistribue :
une pièce ajoutée ne dépasse jamais le plafond, elle prend des triangles aux autres pièces
(les visages sont protégés, `CUT_WEIGHT`). LOD0 actuels : 9 380-11 872 à pied (plusieurs
recettes déjà au plafond : infantry_2/4, archer_4, standard_0) ; LOD1 1 273-1 324 (au
plafond). Coût annoncé = triangles de la pièce au LOD0 (LOD1 entre parenthèses).

Mêmes os et mêmes clips obligatoires : toute pièce est liée rigidement (`bind_rigid`,
`bind_by`) ou coquille du corps (poids du corps).

## Tableau des écarts

| # | Recettes / unités | Écart constaté (référence → FG2) | Effort | Coût LOD0 (LOD1) | Valeur |
|---|---|---|---|---|---|
| 1 | infantry_1 (piquiers), infantry_4 (schiltron), infantry_5 (goedendag), archer_1, archer_2, archer_4 (arbalétriers), cavalry_1 (sergents montés) | **Camail sous le chapel** : la planche montre chapel de fer + camail de mailles (arbalétrier, sergent, milicien) ; FG2 : chapel seul, cheveux et oreilles nus | faible (réemploi `fe.aventail`, bord haut réglable) | ≈ 700 (≈ 60) | forte : silhouette « soldat » vs « paysan au chapeau » |
| 2 | infantry_0, cavalry_0, standard_0/1 (harnois début), infantry_7, cavalry_3 (tardif) | **Gantelets** : gantelets de plates à manchette évasée ; FG2 : gants de cuir (début), mains de plates sans manchette (tardif) | faible (coquille `plate_shell` au poignet, matière des mains) | 0 net (dans le rôle `plates`, 1 500) | moyenne-forte (premier plan, mains toujours visibles) |
| 3 | infantry_1, 4, 5, 6, cavalry_1, 4, 6 (sergents, coutiliers, hobelars) | **Gants de cuir** chez les sergents à arme d'hast ; FG2 : mains nues | nul (matière des mains) | 0 | moyenne |
| 4 | toutes les recettes à pied hors noblesse (infantry_1-6, 8, archer_0-5, musician_0/1) ; dague seule pour le harnois | **Ceinture garnie** : bourse, dague (rognon / baselarde) ; FG2 : ceinture nue (sauf fourreau d'épée FG0, carquois) | faible (deux pièces rigides sur `Hips`) | ≈ 110 (0, LOD0 seul) | moyenne : lecture « équipé » à 5-12 m |
| 5 | archer_0, archer_3 (archers), infantry_2 (milice) | **Bocle à la ceinture** : bouclier rond à umbo pendu à la hanche gauche ; FG2 : absent | faible (disque bombé + umbo, rigide `Hips`) | ≈ 150 (≈ 24) | moyenne-forte (signature de l'archer anglais) |
| 6 | infantry_1, 2, 5, archer_0-2, 4, cavalry_2, musician_0/1 | **Chausses de couleurs variées** (vert, brun, bleu) ; FG2 : même brun `HOSE` partout, le shader ne fait que des nuances (6 teintes multiplicatives proches du neutre) | nul par recette (couleur) ; par soldat = shader (SR2) | 0 | moyenne (masse de bataille moins uniforme) |
| 7 | archer_1 | **Pavois au dos** de l'arbalétrier ; FG2 : seulement archer_2 (génois) et archer_4 (gascons) | faible (pièce existante) mais drapeau de rang BV3 (bit 7) et tir : à revoir avec le jeu | ≈ 200 (≈ 30) | faible : déjà vrai pour les Génois |
| 8 | infantry_1, archer_1, archer_2, musician_* (gamboison) | **Gamboison long à manches** : la coque `tunic` a des manches jusqu'au poignet et la jupe descend à mi-cuisse (`knee_z + 0,14`) — conforme ; seul manque le **col montant** | moyen (coque du cou) | ≈ 150 | faible |
| 9 | infantry_0, cavalry_0, infantry_7 | **Cotte/jupon court cintré** par ceinture basse : présent (surcot FG0, ceinture de hanches à 7 cm sous la taille) — conforme | — | — | — |
| 10 | tous | **Bottes / houseaux de cuir** montant au mollet ; FG2 : souliers pointus FG0 bas | moyen (coque de la jambe basse, masque de la couleur de chausse) | ≈ 300 (dans `legs`) | moyenne (planche : toutes les bottes montent au mollet) |
| 11 | cavalry_0, cavalry_3 (v2), standard_1 | **Caparaçon jusqu'aux jarrets** : ourlet à 0,48 m (jarret ≈ 0,55-0,6 m) — conforme ; robes baie/noire par teinte du shader — conforme ; harnais de cuir (FG4) — conforme | — | — | — |
| 12 | tous | Salissure, étoffes passées, acier mat (indices 1-3) | → SR1/SR2 (shader et matières), hors SR3b | 0 | — |

## Choix SR3b (valeur forte, risque faible)
Retenus : **1** (camail sous le chapel), **2** (manchettes de gantelets), **3** (gants de
cuir), **4** (bourse et dague), **5** (bocle à la ceinture), **6** (chausses par recette).
Écartés pour ce lot : 7 (touche le comportement de rang BV3), 8 (faible), 10 (moyen, à faire
avec SR2 qui salit le bas des jambes), 11 déjà conforme, 12 hors lot.

Les additions vivent dans `battle_fine_sr.py` (tables par recette, pièces rigides) et
n'altèrent pas les recettes Quaternius (`battle_skinned_figures`, rendu `--coarse-figures`).

## Résultat (30/09)
Triangles LOD0 avant → après : archer_0 10 926 → 11 260, archer_1 10 210 → 10 862, archer_2
11 422 → 11 879, archer_3 9 776 → 10 110, archer_5 9 471 → 9 615, infantry_1 11 069 → 11 722,
infantry_3 9 862 → 10 006, infantry_5 11 131 → 11 783, musician_0/1 +144 ; recettes déjà au
plafond (infantry_0/2/4/6/7/8, archer_4, standard_0, montés) inchangées à ± 35 (`fit_budget`
redistribue). LOD1 et LOD2 inchangés (± 15). Pièces LOD0 : bourse 46-48, dague 94-104, bocle
190 (LOD1 ≈ 40), camail ≈ 700 (LOD1 ≈ 100) ; manchettes dans le rôle `plates` (1 500).
