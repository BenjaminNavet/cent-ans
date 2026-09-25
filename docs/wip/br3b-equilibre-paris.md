# BR3b — équilibre du siège de Paris après BR3

Addendum « BR3b » à l'ADR 0047. Branche d'agent `worktree-agent-a212c40bf9fdb8559` (depuis main 21f2b125).

## Objectif
Paris 8-12/20 (victoires de l'assaillant, sonde `br3_assault_probe`), générique 15-19/20, Rouen ≤ 3/20,
autres villes emblématiques ±3/20 par rapport à la référence, maisons brûlées à Paris ≤ 25 en moyenne ;
tests BR3/SG3 verts.

## Sonde
`core/crates/sim-battle/tests/br3_assault_probe.rs` :
- `probe_town_assaults` : `SEEDS`, `LIMIT_S`, `FORT` (2), `BREACH` (40), `TOWNS=generic,paris,...`
  (toute ville emblématique avec un bloc `siege.battle`) ; `BR3_TRACE=1` ajoute un tableau par ville
  et par issue (ouverture, entrée, place, déroutes dedans / sous la chaleur, moral perdu à la
  chaleur, régiments bloqués) ; `BR3_VERBOSE=1` une ligne par partie et par déroute.
- `probe_assault_map` : carte ASCII (`TOWN`, `SEED`, `SNAP_T=t1,t2`) : murs, îlots (en feu,
  brûlés), zone de chaleur, place, régiments, chemin A* d'un régiment de l'assaillant vers la place.

## Référence sur main 21f2b125 (20 graines, brèche 40 %, fort. 2, deux IA)

| Ville | Îlots | Victoires assaillant | Durée médiane (s) | Pertes assaillant | Pertes garnison | Maisons brûlées (moy.) |
|---|---|---|---|---|---|---|
| générique | 59 | 17/20 | 438 | 55 | 197 | 10.9 |
| paris | 61 | **17/20** | 501 | 85 | 194 | 6.7 |
| rouen | 78 | 2/20 | 307 | 91 | 148 | 5.6 |
| avignon | 80 | 18/20 | 452 | 154 | 175 | 5.0 |
| bordeaux | 85 | 18/20 | 463 | 70 | 196 | 17.4 |
| bruges | 82 | 19/20 | 413 | 71 | 159 | 5.0 |
| calais | 84 | 6/20 | 329 | 78 | 152 | 3.4 |
| london | 70 | 18/20 | 501 | 81 | 208 | 4.3 |

Fortification 4 : générique 15/20, Paris 13/20, Rouen 0/20. Sans brèche : 18/18/2.

Sonde SG3 (`sg3_assault_probe`, armées de la démo, 10 graines) : Paris 10/10, Avignon 8/10,
Bruges 10/10, Calais 10/10, Rouen 8/10.

**Le 5/20 de BR3 n'existe plus sur main** : SG3 (PV de la porte et des murs dans
`siege_works.json`) l'a effacé. Preuve : en remettant dans `siege_works.json` les valeurs d'avant
SG3 (mur 500 × (1 + fort.), porte 250 × (1 + fort.), bélier 4 PV/s, engins 1,6), la sonde redonne
exactement les chiffres de BR3 (générique 17/20, Paris 5/20, Rouen 1/20, 20,9 maisons brûlées).

## Diagnostic

Avec les ouvrages d'avant SG3 (reproduction du 5/20) : les défaites de Paris sont des déroutes
**dans la ville, sous la chaleur** (6,7 déroutes dedans sur 10 par partie, 4,9 sous la chaleur ;
91 points de moral perdus à la chaleur par l'assaillant). Carte (graine 2, 360-475 s) : la brèche
s'ouvre dans l'angle est de l'enceinte ; les traits incendiaires des engins (qui visent le chemin de
ronde, +18 m de dépassement, portée d'allumage 30 m) ont mis le feu aux rangées adossées au rempart
juste derrière la brèche ; les milices (moral 40) et les archers (55) entrent en tête, se battent
dans la zone de chaleur (0,5 point de moral/s à pleine intensité) et se débandent en cascade vers
20 de moral avec presque tous leurs hommes. Aucun régiment bloqué, chemins A* normaux.

Sur main (ouvrages SG3) : la porte tombe vers 250-270 s avec peu de maisons en feu (6 en
moyenne) ; les 3 défaites de Paris sont des escalades tardives (ouverture à 515 s) qui se débandent
au pied du mur, sans chaleur (moral perdu à la chaleur ≈ 0).

## État
- [x] Référence sur main (sondes BR3 et SG3), toutes villes emblématiques.
- [x] Diagnostic instrumenté (traces, carte ASCII).
- [ ] Contre-épreuve : SG3 sans BR3 (pointe de la branche SG3 d29e5684).
- [ ] Décision / correctif minimal.
- [ ] Mesures après, addendum ADR 0047, codex si chiffres visibles changent.
- [ ] Vérifications (fmt, clippy, test, pytest, build.sh, smoke siège), fusion de main.

## Prochaine étape
Contre-épreuve SG3 sans BR3, puis décider (Paris est déjà au-dessus de la cible).
