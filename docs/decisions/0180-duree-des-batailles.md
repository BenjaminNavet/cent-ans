# 0180 — Durée des batailles : mesure, cadence en données, et limite des leviers de combat

Date : 2026-10-03 (lot A6-L13, constat B1 de `docs/audit/a6-audit-joueur.md`).
Statut : **décision retenue (piste 1)** : écart de déploiement ×1,5 et mêlée ralentie (voir « Décision retenue »).

## Contexte

Le joueur trouve les batailles 3D trop courtes (2 à 5 min de jeu, contre 10 à 20 min dans Total War) et
manque de temps pour manœuvrer. Le lot demandait d'allonger la durée jusqu'à la déroute d'un camp de ×2 à ×3
(médiane visée 8-12 min simulées) **en jouant uniquement sur les données de combat et de moral**, sans changer
le vainqueur (accord ≥ 90 %) ni la sonde `auto_resolve_calibration`.

## Ce qui est livré

1. **Sonde de durée** `core/crates/sim-battle/tests/l13_duration.rs` (ignorée, sans rendu, IA contre IA) :
   les 20 scénarios de la fixture de calibration de l'auto-résolution, une bataille de campagne
   « 600 contre 550 mixte » (≈ 7 régiments de 85 hommes par camp), et les trois batailles historiques
   (Crécy, Azincourt, Poitiers), 6 graines chacun. Elle écrit une ligne TSV par bataille (vainqueur, durée,
   pertes, instants du premier contact et de la première déroute) et la médiane par scénario.
   `l13_trace.rs` (ignorée) imprime l'évolution d'une bataille toutes les 10 s (effectifs, régiments en déroute
   ou en mêlée, moral moyen, journal). Les deux acceptent des surcharges par variables d'environnement
   (`L13_PACE`, `L13_ROUT`, `L13_PUSH`, `L13_DECISION`, `L13_AMMO`, `L13_SPEED`) pour balayer les réglages sans
   recompiler.
2. **Cadence du combat en données** : `data/rules/battle_pace.json` (schéma `battle_pace_rules.schema.json`,
   `sim-battle/src/pace.rs`). Les constantes codées en dur `MELEE_RATE` (0,035), `RANGED_RATE` (0,3),
   `LOSS_MORALE_FACTOR` (60), le moral perdu par seconde sous flanc (1,5) et revers (3), le seuil de déroute (20)
   et le seuil de ralliement (40) y sont sortis, séparément pour les batailles rangées (`field`) et les sièges
   (`siege`, valeurs d'origine). `BattleSim::set_pace` / `set_rout_rules` servent aux sondes. Aucun changement
   de comportement : la sonde donne les mêmes résultats qu'avant.

## Mesures (avant, valeurs actuelles)

Temps simulé jusqu'à la fin de la bataille (déroute générale), médiane sur 6 graines :

| Cas | Durée | Premier contact |
|---|---|---|
| Campagne 600 contre 550 mixte | 277 s | 162 s |
| 20 scénarios de la fixture (médiane des médianes) | 302 s | de 52 s à 491 s |
| Crécy (carte historique) | 839 s | 580 s |
| Azincourt (carte historique) | 843 s | 159 s |
| Poitiers (carte historique) | 924 s | 178 s |

Le premier contact arrive à 160-260 s dans les petits scénarios : la marche d'approche pèse de 50 à 80 % du
temps total. La mêlée, du premier contact à la déroute générale, ne dure que 70 à 120 s.

## Ce que les leviers de combat permettent

Quatre familles de leviers ont été essayées, seules, puis combinées (balayage de 24 scénarios × 6 graines,
puis recherche aléatoire de 94 combinaisons) :

- létalité de mêlée ÷2 à ÷10, létalité des tirs ×0,3 à ×1 (avec munitions ×1 à ×3) ;
- moral perdu par mort (60 → 6), par flanc (÷2 à ÷7), par poussée/compression (`battle_push.json`, ÷3 à ÷10) ;
- contagion de la déroute (`battle_rout.json`, 0,5 → 0,1 par s), seuil de déroute (20 → 5) ;
- seuil de rupture de l'armée (40 % → 25 %) et délai de maintien (30 → 90 s).

Résultats :

- Un levier isolé change peu la durée (×1,0 à ×1,25) : létalité de mêlée ÷2 ou poussée ÷3, 24/24 vainqueurs
  conservés mais +3 % de durée ; seuil de rupture 25 %, +4 %.
- Les combinaisons fortes plafonnent à **×1,3 à ×1,4** (médiane des médianes 274 → 360-385 s) tout en gardant
  ≥ 21/24 vainqueurs. Les pires extrêmes (mêlée ÷10, tirs ÷3, moral ÷10, contagion ÷5) n'atteignent que ×1,8
  (≈ 500 s) et perdent un tiers des vainqueurs.
- **Le vainqueur historique bascule dès que le moral devient plus résistant** : à Crécy, Azincourt et Poitiers
  l'issue (victoire anglaise dans 100 % des graines) repose sur la panique des chevaux, les pieux et la
  contagion de la déroute française. Dès que la contagion est réduite d'un tiers, la France gagne 3 à 6 graines
  sur 6 ; aucune des 94 combinaisons aléatoires ne tient à la fois ≥ 22/24 vainqueurs et Crécy anglais.
- Alléger la létalité des tirs profite à l'attaquant (la marche d'approche dure autant, la volée tue moins) :
  Crécy, les « Français contre Anglais » de la fixture basculent en faveur de la France.
- La cause structurelle : la rupture d'une armée de 7 régiments vient d'une cascade (un régiment cède, la
  contagion et la chute des étendards entraînent les voisins en 20 à 60 s), non d'une usure par les pertes.
  Les régiments cèdent avec 1 à 15 % de pertes. Ralentir la mort ne retarde donc pas la déroute, et ralentir le
  moral retarde la déroute mais fait basculer le rapport de force historique.
- Le temps de marche d'approche est le plus gros poste et n'est pas une donnée de combat : réduire la vitesse
  des régiments de 40 % (`stats.speed` × 0,6, sans autre changement) donne ×1,4 avec **24/24 vainqueurs
  conservés** ; à ×0,4 la durée ne progresse plus et 4 vainqueurs basculent (arrondi de la vitesse, tirs plus
  longs pour les archers).

## Pistes examinées

La cible ×2 à ×3 avec accord ≥ 90 % des vainqueurs **n'est pas atteignable par les seules données de combat et
de moral**. Trois pistes ont été examinées :

1. **Allonger la marche d'approche** (distance de déploiement, vitesse de marche des régiments) : ×1,4 mesuré
   pour 24/24 vainqueurs, cumulable avec un léger adoucissement de la mêlée (létalité ÷2, poussée ÷3 :
   ×1,05, vainqueurs conservés) pour ≈ ×1,6.
2. **Régiments plus nombreux et armées plus larges** (effectifs par régiment, nombre de régiments par camp) :
   la rupture à 40 % des régiments valides vient plus tard avec plus de régiments, et la cascade est moins
   brutale. Ce levier relève des données d'unités et du déploiement, hors périmètre du lot.
3. **Accepter un nouvel équilibre historique** : si la bascule de Crécy/Azincourt/Poitiers vers la France est
   acceptable, la combinaison `b=0,30 a=0,67 L=0,32 F=0,8 P=0,53 C=1,13` (létalité mêlée ×0,30, tirs ×0,67,
   moral par mort ×0,32, flanc ×0,8, poussée ×0,53, contagion ×1,13, munitions ×1,5) donne ×1,4 avec 21/24
   vainqueurs et Crécy 1 graine française sur 6. Il faudrait alors réétalonner `ep7_historical` et la
   calibration de l'auto-résolution (`balance_probe rt`).

## Décision retenue

Piste 1, par la **distance de déploiement** (et non la vitesse, pour ne pas ralentir la démarche des
régiments) :

- `data/rules/battle_scale.json` : `line_gap_m` ×1,5 (300 → 450, 340 → 510, 380 → 570), les terrains et les
  profondeurs de zone ne changent pas (lignes à 175/625 m sur le champ de 800 m, zones bornées par les marges).
  La valeur était déjà en données. Les cartes historiques gardent leur géométrie.
- `data/rules/battle_pace.json` : `field.melee_rate` 0,035 → 0,0105 (×0,3) ; tout le reste (tirs, moral,
  contagion, seuils) inchangé. C'est la meilleure combinaison douce trouvée qui garde 23/24 vainqueurs et
  Crécy, Azincourt, Poitiers anglais sur 6 graines sur 6.

| Mesure (médiane, 6 graines) | Avant | Écart seul | Écart + mêlée ×0,3 |
|---|---|---|---|
| Médiane des médianes (24 cas) | 274 s | 337 s (×1,23) | 349 s (×1,27) |
| Campagne 600/550 | 277 s | 315 s | 352 s (×1,27) |
| Vainqueurs conservés | 24/24 | 24/24 | 23/24 (« Crécy » de la fixture : France 3 graines sur 6) |
| Crécy / Azincourt / Poitiers (anglais) | 6/6 chacun | 6/6 | 6/6 chacun |
| Accord avec la référence 3D de la fixture | 17/20 | 17/20 | 18/20 |

Le gain total reste ×1,27, sous le ×1,6 visé : l'écart de lignes ne déplace que la marche d'approche (≈ 20 %
du total mesuré sur la fixture) et les cartes historiques ne la changent pas. Aller plus loin demande des
armées plus larges (piste 2) ou d'accepter la bascule historique (piste 3).

## Munitions

Sans changement de létalité des tirs, les munitions actuelles (20 à 48 volées) suffisent à la durée mesurée.
Si la piste 3 est retenue, multiplier `ammo` par l'inverse du facteur appliqué à `ranged_rate`
(1,5 pour 0,67), dans `data/unit_types/`, et non dans le code (l'auto-résolution n'utilise pas les munitions).

## Reproduire

```
cd core
L13_OUT=/tmp/l13.tsv cargo test --release -p sim-battle --test l13_duration -- --ignored --nocapture survey
L13_TRACE=campagne cargo test --release -p sim-battle --test l13_trace -- --ignored --nocapture
```
