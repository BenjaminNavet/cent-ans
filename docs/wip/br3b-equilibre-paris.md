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

### Contre-épreuve : SG3 sans BR3

Pointe de la branche SG3 (d29e5684, ville d'avant BR3 + ouvrages SG3), même sonde, 20 graines :
générique 19/20, **Paris 11/20**, Rouen 0/20. Donc sur main, BR3 fait passer Paris de 11 à
17/20 : l'objectif 8-12 garde son sens, mais dans l'autre sens (Paris est devenue trop facile).

### Mécanisme (main)

À Paris tout se joue entre le bélier et les arbalétriers du rempart (traces `BR3_RAM=1`) :
l'équipage du bélier (12 hommes) meurt sous les carreaux des arbalétriers sur le mur ; dans les
défaites la porte reste à 1-4 PV sur 460. Sur main, dans 14 parties sur 20, un régiment
d'arbalétriers du rempart se débande avant 120 s : les engins visent le pan de mur où il se tient,
le dépassement de 18 m allume la rangée adossée au rempart (11,3 m de la ligne du mur), et la
chaleur (0,5 point de moral/s) le fait fuir. Le rectangle du régiment tourne avec son orientation
(il vise en biais) et s'enfonce jusqu'à 1 m de l'îlot en feu. Avant BR3 il n'y avait pas de
rangées contre le rempart. Expérience : sans chaleur pour les régiments sur le mur, Paris tombe à
3/20 ; avec un rectangle aligné sur le mur (quelle que soit l'orientation), 3/20 aussi.

### Balayages (générique / Paris / Rouen / Calais, 20 graines)

Chaleur mesurée au rectangle des îlots (et non plus au disque) : aucun effet (17/17/2/5).

| Réglage | Générique | Paris | Rouen | Calais |
|---|---|---|---|---|
| référence main | 17 | 17 | 2 | 6 |
| moral à la chaleur 0,35 /s | 18 | 16 | 4 | 6 |
| moral à la chaleur 0,25 /s | 18 | 16 | 1 | 10 |
| moral à la chaleur 0,15 /s | 19 | 16 | 0 | 8 |
| rayon de chaleur 6 m | 19 | 16 | 0 | 3 |
| rayon de chaleur 4 m | 19 | 15 | 0 | 4 |
| dépassement d'allumage 10 m | 19 | 17 | 3 | 5 |
| chaleur décroissante avec la distance | 19 | 14 | 0 | 4 |
| pas de chaleur sur le rempart | 18 | 3 | 0 | 6 |
| rectangle du régiment aligné sur le mur | 18 | 3 | 1 | 6 |
| chemin de ronde des villes emblématiques 14 m | 17 | 15 | 0 | 7 |

Paris est **bimodale** : avec le rectangle aligné, les 17 défaites sont identiques (équipage du
bélier tué par les arbalétriers du rempart, porte à 1-4 PV sur 460, escalade ratée à 515 s) ; les
3 victoires sont les graines où la porte tombe à 254 s. Le résultat se joue sur un seul régiment :
les arbalétriers près de la porte, rompus ou non par la chaleur de la rangée adossée au rempart.
Les réglages en tout ou rien donnent 3/20 ou 15-17/20. Levier progressif à l'essai : une part
seulement de la chaleur atteint le rempart (hauteur et parapet).

### Autres leviers écartés

Rayon de l'anneau de Paris (`siege.battle.radius_m`, 150 m par défaut) : 135 m → 20/20, 175 m →
20/20 (la géométrie change tout, pas de pente exploitable).

## Correctif

1. **Chaleur sur le chemin de ronde** (`data/rules/siege_fire.json`, `heat.wall_walk_factor` =
   0,3 ; schéma ; `sim/fire.rs` `heat_on`, exposé en `BattleSim::heat_intensity`) : un régiment sur
   le rempart, au-dessus de la rue et derrière le parapet, ne reçoit que 30 % de la chaleur des
   maisons en feu (pertes et moral). Balayage du facteur (Paris / Bordeaux / Londres) : 1 → 17/18/18,
   0,6 → 15/15/15, 0,45 → 14/12/16, 0,3 → 12/12/16, 0,2 → Paris 12.
2. **IA d'assaut** (`ai.rs`, `plan_siege_attack`) : défaut révélé par (1). Un régiment de
   l'assaillant déjà passé par-dessus le mur (dedans ou sur le rempart), sans porte ni brèche
   ouverte, recevait encore l'ordre de gagner son point d'échelle — 25 m derrière le mur, donc là
   où il était : il restait planté jusqu'à la nuit (Rouen de la sonde SG3 : 6 nuls à 1800 s sur 10
   avec (1) ; c'était aussi le point ouvert d'Avignon dans SG3). Il marche maintenant vers la place.
   Sans effet sur la sonde BR3 (chiffres identiques avec et sans).
3. Tests : `fire.rs::the_wall_walk_is_sheltered_from_the_heat`,
   `sg1.rs::an_attacker_over_the_wall_makes_for_the_square` (échoue sans le correctif : 124 → 123 m).
4. Codex `cdx_jeu_incendies` : la part de chaleur sur le chemin de ronde.

## Mesures après (sonde BR3, 20 graines, brèche 40 %, fort. 2, deux IA)

| Ville | Îlots | Victoires assaillant avant → après | Durée médiane (s) | Pertes assaillant | Pertes garnison | Maisons brûlées (moy.) |
|---|---|---|---|---|---|---|
| générique | 59 | 17 → 18/20 | 438 → 441 | 55 → 56 | 197 → 197 | 10.9 → 10.4 |
| paris | 61 | 17 → **12/20** | 501 → 560 | 85 → 78 | 194 → 166 | 6.7 → 8.9 |
| rouen | 78 | 2 → 0/20 | 307 → 305 | 91 → 70 | 148 → 157 | 5.6 → 4.0 |
| avignon | 80 | 18 → 18/20 | 452 → 452 | 154 → 154 | 175 → 175 | 5.0 → 5.0 |
| bordeaux | 85 | 18 → **12/20** | 463 → 470 | 70 → 85 | 196 → 212 | 17.4 → 19.9 |
| bruges | 82 | 19 → 19/20 | 413 → 406 | 71 → 69 | 159 → 167 | 5.0 → 4.7 |
| calais | 84 | 6 → 6/20 | 329 → 334 | 78 → 90 | 152 → 167 | 3.4 → 2.9 |
| london | 70 | 18 → 16/20 | 501 → 501 | 81 → 77 | 208 → 169 | 4.3 → 5.3 |

Paris sur 40 graines : 26/40 (13/20). Défaites de Paris : l'équipage du bélier tué par les
arbalétriers du rempart, escalade tardive (515 s) qui se débande ; victoires : porte tombée ou
place tenue.

Sonde SG3 (armées de la démo, 10 graines), victoires de l'assaillant main → chaleur seule → après :
Paris 10 → 9 → 10, Avignon 8 → 8 → 10, Bruges 10 → 10 → 10, Calais 10 → 10 → 10, Rouen 8 → 4 → 10 ;
durées médianes après : 240, 294, 285, 262, 329 s (main : 233, 421, 243, 262, 546).

## Mesures finales après fusion de main (e885d011)

Main seul = 034351af (archive temporaire, supprimée) ; main + BR3b = e885d011. Sonde BR3,
20 graines, brèche 40 %, fortification 2, deux IA.

| Ville | Victoires assaillant main → BR3b | Durée médiane (s) | Pertes assaillant | Pertes garnison | Maisons brûlées (moy.) |
|---|---|---|---|---|---|
| générique | 18 → 19/20 | 431 → 434 | 51 → 52 | 194 → 195 | 10.8 → 10.4 |
| paris | 17 → **12/20** | 501 → 550 | 80 → 73 | 186 → 160 | 6.5 → 8.8 |
| rouen | 2 → 2/20 | 307 → 305 | 93 → 72 | 147 → 151 | 5.8 → 4.3 |
| avignon | 19 → 19/20 | 447 → 447 | 155 → 154 | 172 → 171 | 4.8 → 4.8 |
| bordeaux | 17 → **12/20** | 452 → 471 | 69 → 86 | 194 → 212 | 17.4 → 20.1 |
| bruges | 20 → 20/20 | 404 → 403 | 65 → 65 | 151 → 157 | 4.7 → 4.7 |
| calais | 7 → 7/20 | 330 → 332 | 79 → 90 | 152 → 166 | 3.4 → 3.0 |
| london | 17 → 18/20 | 500 → 502 | 80 → 76 | 204 → 168 | 4.3 → 5.3 |

Sonde SG3 (armées de la démo, 10 graines), main → BR3b : Paris 10 → 10/10 (230 → 232 s),
Avignon 10 → 10 (413 → 293 s), Bruges 10 → 10 (243 → 285 s), Calais 10 → 10 (262 → 262 s),
Rouen 8 → 10 (546 → 329 s).

Objectifs : Paris 12/20 ✓ (8-12 ; 13/20 sur 40 graines avant fusion), générique 19/20 ✓, Rouen
2/20 ✓, maisons brûlées à Paris 8,8 ✓, autres villes à ±1 sauf **Bordeaux −5** ✗.

## Écart à l'objectif : Bordeaux

Bordeaux perd 5 victoires (17 → 12/20) : la même garnison sur le rempart y est soulagée de la
chaleur. Aucun réglage global ne sépare Paris de Bordeaux (balayage avant fusion : facteur 1 →
17/18, 0,6 → 15/15, 0,45 → 14/12, 0,3 → 12/12). Options : (a) garder 0,3 (retenu : Paris dans la
cible, Bordeaux 12/20) ; (b) 0,6 (toutes les villes à ±3, Paris 15/20) ; (c) 1,0 (Paris 17/20),
en ne gardant que le correctif d'IA ; (d) un levier propre à Paris ou à Bordeaux (le bloc
`siege.battle` n'offre que murs, porte, rues, place, rayon ; le rayon est chaotique : Paris à
135 m ou 175 m → 20/20).

## État
- [x] Référence sur main (sondes BR3 et SG3), toutes villes emblématiques.
- [x] Diagnostic instrumenté (traces, carte ASCII).
- [x] Contre-épreuve : SG3 sans BR3 (pointe de la branche SG3 d29e5684).
- [x] Correctif : chaleur sur le chemin de ronde (données) + IA d'assaut (défaut révélé).
- [x] Mesures après, addendum ADR 0047, codex.
- [x] Fusion de main (e885d011), mesures finales main seul / main + BR3b.
- [ ] Vérifications après fusion (fmt, clippy, test, pytest, build.sh, smoke siège).

## Prochaine étape
Vérifications après fusion, rapport.
