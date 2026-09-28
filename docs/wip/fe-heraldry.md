# FE — moteur de blasons (heraldry.py), meubles manquants

## État : terminé

`uv run --project tools pytest -q tools/tests/test_heraldry.py tools/tests/test_heraldry_houses.py`
passe (11 tests, dont un nouveau test de non-régression).

## Ce qui a été fait

Ajout au moteur (`tools/cent_ans_tools/heraldry.py`) de dessins simples pour les
meubles absents, qui faisaient retomber ces blasons sur un champ plein et
entraient en collision entre eux :

- `_draw_key_single` : une clef seule posée en pal (`fac_bremen`).
- `_draw_cauldrons` : chaudières échiquetées or/gueules, empilées en pal
  (`fac_lara`).
- `_draw_hand` : main dextre coupée (`fac_tyrone`).
- `_draw_ship` / `_draw_ship_shape` : nef (lymphad), aussi câblée dans la
  grammaire v2 (`PIECES["nef"]`, `_charge_mask`) pour la maison `macdonald`
  (`fac_isles`).
- `_draw_crescent` : croissant décroissant (`fac_luna`).
- `_draw_ox` : bœuf silhouette simple (`fac_urgell`).
- `_draw_chief` : chef, avec variante denché (dents) si `blazon.has("denche")`
  (`fac_ormond`).

Nouvelle méthode `Blazon.tincture_near(word)` : comme `tincture_after`, mais
tolère jusqu'à 40 caractères de qualificatifs entre le mot-clé et la
tincture (« la main dextre coupée **de gueules** », « la nef (lymphad)
**d'argent** »). Utilisée pour ces nouveaux meubles au lieu de
`blazon.charge` (secondary_color), car pour `fac_bremen` la tincture du
blason (gueules) ne correspond pas à `secondary_color` (argent) — donnée
volontairement approximative (armes « incertaines »), non modifiée.

Placement des nouvelles branches dans la chaîne `elif` de `_draw_charges` :
vérifié qu'aucune ne capte un blason existant qui contient le même mot en
position secondaire (piège « en sautoir » déjà documenté) : `chef` placé
après `hermine`/`pals` (attrape déjà `fac_bamberg`, `fac_blois`), `clef`
placé après `clefs` (attrape déjà `fac_papacy`).

## Doublon de maison trouvé

`test_every_house_blazon_renders` échouait sur `macdonald` (nef, non
dessinée) vs `virneburg` (« trois tours d'argent », mot-clé `tour` absent de
la grammaire v2) : les deux retombaient sur un champ d'azur plein identique.
Résolu en ajoutant `nef` à la grammaire v2 ; `virneburg` n'a pas été touché
(pas dans le périmètre de la tâche — ses tours restent non dessinées mais ne
collisionnent plus avec personne d'autre après le fix de `macdonald`).

## Test de non-régression

`tools/tests/test_heraldry.py::test_new_charges_render_distinct_from_plain_field` :
pour les 7 blasons de factions listés, vérifie que le rendu diffère d'un
champ plein de mêmes tinctures.

## Assets régénérés

`uv run --project tools cent-ans assets heraldry` régénère tous les écus
(beaucoup de factions/maisons de ce chantier FE n'avaient jamais été
committées côté assets sur cette branche — hors périmètre). Seuls les
fichiers liés à la tâche ont été ajoutés au commit :
`game/assets/heraldry/fac_{bremen,lara,tyrone,isles,luna,ormond,urgell}.png`
et `game/assets/heraldry/houses/{macdonald,lara,luna,aragon_urgell}.png`.

## Points ouverts

- `virneburg` (« trois tours ») reste sans meuble dessiné (champ plein) ;
  hors périmètre de cette tâche, à signaler si une future tâche houses/FE
  s'en occupe.
- De nombreux autres écus de factions/maisons de ce chantier FE (plus
  larges que les 7 demandés) n'ont pas d'asset PNG committé sur cette
  branche ; laissés non trackés (`git status` les montre en `??`), pas
  touchés ici pour ne pas empiéter sur le travail d'autres agents.
