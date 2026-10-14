# CC C0a — sélection des lieux et fusion des provinces

Module `tools/cent_ans_tools/compose/select.py`, commande `cent-ans compose select [--scope owner|realm]`,
tests `tools/tests/test_compose_select.py`, schémas `compose_selection` / `compose_id_map`.
Sorties : `data/map/compose/selection.json` + `id_map.json` (scope `owner`, spec littérale) et
`selection_realm.json` + `id_map_realm.json` (scope `realm`). Aucune donnée de jeu modifiée.

## Blocage de conception : la cible de 180 provinces est inatteignable avec « même owner »
Les 419 provinces gardées appartiennent à 175 factions, mais seules 263 sont adjacentes à une province de
même owner : 156 provinces sont isolées, donc **210 composantes connexes au minimum (232 avec le plafond de 5)**.
Résultat réel en scope `owner` : **238 provinces** (cible 180 hors d'atteinte). Les voisines viennent de
`provinces.geojson` (le champ `neighbors` des `data/provinces/*.json` est vide pour 437 provinces sur 443).
Option proposée sans trancher : fusion entre provinces de même *suzerain final* (`factions.suzerain`, scope `realm`) :
borne théorique 156, résultat glouton **208** (les 175 capitales de faction ne peuvent être absorbées ni fusionnées entre elles).
Pour atteindre ~180 il faudra au concepteur de choisir : realm, plafond de groupe > 5, ou cible plus haute (~230 en owner).

## Emprise
Retirées (24 provinces) : capitale à lon > 41.5, plus les tags `horde` (tout), `caucase`, `mesopotamie`.
Tags gardés : Levant, Anatolie, Égypte, Maghreb, Crimée, Rus' (Moscou jusqu'à Vladimir). Factions sans territoire restant (10) :
alania, artuqids, bashkir, circassia, georgia, jalayirids, mordva, perm, samtskhe, viatka.
`fac_golden_horde` garde la Crimée mais perd sa capitale (Saray) : à traiter en C3.

## Comptes (scope owner / realm)
| | owner | realm |
|---|---|---|
| provinces | 238 | 208 |
| lieux | 400 (238 cities, 120 towns, 32 castles, 9 abbeys, 1 village) | 400 (208/143/34/10/5) |

Par zone (provinces / lieux, owner) : France 29/65, Îles Britanniques 17/36, Ibérie 18/31, Italie 26/38,
Empire 40/61, Scandinavie 8/15, Est 48/68, Balkans-Anatolie-Levant 43/69, Maghreb (+Égypte) 9/17.
Le nombre de cités est fixé par celui des provinces (1 chacune) : il ne reste que ~160 lieux secondaires.

## Protégés manquants (quota de 4 secondaires atteint ou espacement)
Aucun id de test manquant. Absents (owner) : boston, clairvaux, colchester, hastings, maldon, mont_saint_michel,
portsmouth, rye, scarborough, vincennes, whitby, winchelsea, windsor (ports/places de la côte anglaise regroupés dans quelques provinces).

## Écart au plus proche voisin (km, owner)
0-15 : 8 ; 15-30 : 42 ; 30-45 : 57 ; 45-60 : 77 ; 60-90 : 90 ; 90-120 : 55 ; 120-200 : 60 ; >200 : 11.
107 lieux sont à moins de 45 km d'un autre : cités voisines et lieux protégés, exemptés d'espacement.

## Douteux
- Un lieu non protégé n'est retenu qu'à ≥ 45 km de tous les retenus ; le quota 1-4 est un centile d'importance (40/75/92 %).
- Les cités surnuméraires d'une province fusionnée deviennent `town` (`former_kind: city`) quand elles sont gardées.
- `selection*.json` dépend de `data/`: à régénérer si les lieux changent.
