# FE4e — Registre féodal de l'Italie (1337)

Branche `feat/fe4e-italie`, issue de `main` (0cdafa9c, qui contient F0-F5).

## État (en cours)

Titres, provinces, factions créés (données brutes, pipeline géo et colonies pas encore faits).

### Entités retenues (justification du périmètre)

14 nouvelles provinces : Mantoue, Ferrare, Saluces, Pise, Sienne, Lucques (occupée par Vérone),
Bologne, Pérouse, Urbin, Rimini, Padoue (occupée par Vérone), Parme (occupée par Vérone), Pavie
(extension de Milan), Tarente. Provinces existantes réassignées à une nouvelle faction propre :
Montferrat (déjà présente, tenue par `fac_empire` par défaut) → `fac_montferrat` ; Sicile/Trinacrie
(`prov_sicilia`, tenue par `fac_aragon`) → `fac_sicily`, royaume aragonais distinct de l'Aragon
continental (branche cadette depuis la paix de Caltabellotta, 1302).

12 nouvelles factions jouables : Mantoue (Gonzague), Ferrare (Este), Montferrat (Paléologue),
Saluces, Pise, Sienne, Bologne (Pepoli), Pérouse, Urbin (Montefeltro), Rimini (Malatesta), Sicile
(Trinacrie), Tarente (Anjou, apanage vassal de Naples).

Choix explicites :
- Lucques, Padoue, Parme : occupées par Vérone (Mastino II della Scala) au printemps 1337 — pas
  de faction jouable propre à cette date (Carrare exilés de Padoue jusqu'en août 1337, après le
  début de partie ; Lucques et Parme prises par Vérone en 1335). Titre propre créé (pour la
  suzeraineté de jure et une future restitution), `holder_1337` = `fac_verona`, marqué `uncertain`.
- Sardaigne : non traitée (mandat F4d, à vérifier dans `main` avant fusion — absente de `main` au
  28/09).
- Achaïe/Morée : hors carte (Péloponnèse), non traitée ; signalée dans la description de Tarente
  (prétention de Catherine de Valois-Courtenay).
- Cremona/Plaisance/Trévise : non ajoutées, pour limiter la portée (Pavie seule ajoutée côté
  Milan, comme extension sans nouvelle faction).

## Prochaine étape
Settlements (≥3 colonies sourcées par nouvelle province), personnages (souverains + héritiers),
héraldique des maisons, cartes front-end, portraits, régénération géo.

## Bogues corrigés en cours de route
- `prov_bologna` : terrain généré par erreur à `continental` (valeur de climat) au lieu de
  `plains`.
- `tit_papacy.de_jure_provinces` et `tit_empire.de_jure_provinces` contenaient déjà
  `prov_bologna`/`prov_ferrara` et `prov_montferrat` (Romagne/Montferrat déjà de jure sous
  Papauté/Empire avant ce lot) : retirés de ces listes, redondants avec les nouveaux titres
  `tit_bologna`/`tit_ferrara`/`tit_montferrat` qui les portent désormais comme domaine propre
  (règle « province dans exactement un titre »).

## Correction majeure (settlements)

En générant les colonies, j'ai découvert que plusieurs provinces existaient déjà dans `main` avec
des colonies « graines » anticipant explicitement ce lot (mêmes id `set_*`, même coordonnées,
descriptions déjà sourcées) : `prov_ancona` (Pérouse, Urbino, Assise, Gubbio en ville/châteaux),
`prov_milano` (Pavie), `prov_firenze` (Pise, Sienne, Lucques), `prov_venezia` (Padoue, Praglia,
Monselice). Repris ces entrées authentiques (au lieu de mes inventions) pour les colonies capitales
des nouvelles provinces, retirées des fichiers parents, poids/population des provinces parentes
réduits en conséquence (même méthode que pour Ferrare/Bologne).

Découverte cruciale : la fiche `set_pavia` authentique précise que Pavie reste une commune gibeline
dominée par la faction Beccaria et « ne tombera aux mains des Visconti qu'en 1359 » — Pavie n'est
donc PAS milanaise en 1337. Corrigé : `fac_pavia` créée (13e nouvelle faction jouable, république
gibeline indépendante, sans suzerain), `tit_pavia` et `prov_pavia.owner` passés de `fac_milan` à
`fac_pavia`.

De même, `set_urbino` authentique dit les Montefeltro « gibelins souvent en lutte contre le pouvoir
pontifical » depuis 1234 : `tit_urbino` corrigé en titre souverain (pas de `de_jure_liege` vers
`tit_papacy`), relation Urbin-Papauté passée de vassal/incertain à guerre.

Total révisé : 15 nouvelles provinces à géométrie propre (+ Montferrat et Sicile réassignées),
13 nouvelles factions jouables (Mantoue, Ferrare, Montferrat, Saluces, Pise, Sienne, Bologne,
Pérouse, Urbin, Rimini, Sicile, Tarente, Pavie).
