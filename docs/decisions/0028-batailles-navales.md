# ADR 0028 — Batailles navales

Date : 2026-09-25. Statut : partiellement remplacé par l’ADR 0201 (le moteur temps réel, les scénarios historiques et la 3D sont supprimés ; modèle de combat, auto-résolution et conséquences de campagne restent). Lot NV1. Revient sur la ligne « Naval : transport maritime
abstrait uniquement, pas de bataille navale » du document de conception (§ 2).

## Contexte

La guerre de Cent Ans s'ouvre sur mer : l'Écluse (1340) donne la Manche à Édouard III,
Winchelsea (1350) la lui conserve, La Rochelle (1372) la rend aux galères castillanes alliées de
Charles V. Entre les batailles, la guerre de course et les descentes sur les côtes. Le jeu ne
connaissait que la traversée de port à port (ADR 0010) : aucune flotte ne pouvait l'empêcher.
Le joueur veut des batailles navales à la Total War : navires 3D, équipages visibles, tir,
grappins, abordages, feu.

## Options

- **Réutiliser `BattleSim` terrestre** avec des « régiments » posés sur des ponts : les règles de
  formation, de terrain et de charge n'ont pas de sens à bord ; il faudrait les neutraliser une à une.
- **Simulation navale séparée dans `sim-battle`** (module `naval`), partageant le générateur
  aléatoire, les événements de tir (`ShotEvent`) et les régiments de campagne (`UnitSetup`).
- **Flottes comme entités de campagne** (déplacement sur les mers, ordres de patrouille) :
  refonte du mouvement libre, de l'IA et de la sauvegarde, hors de portée du lot.

## Décision

1. **Cœur** : `sim-battle::naval`. Un navire (`Ship`) a une coque, un niveau de feu, une hauteur
   de bord (franc-bord) et des châteaux avant et arrière ; son équipage de combat est fait des
   régiments embarqués (archers, arbalétriers, hommes d'armes) et de ses marins. Quatre classes
   en données (`data/naval/ships/`) : cogue, nef, galère, barge. Pas fixe de 0,1 s, déterministe.
   - **Approche au vent** : une voile carrée ne remonte pas à moins de 67° du lit du vent
     (virements de bord) ; la flotte au vent tient l'avantage (portée et justesse +20 % × force
     du vent). Les galères rament sans vent, mais leurs rameurs s'épuisent.
   - **Tir** : volées par régiment, depuis les châteaux (hauteur) et le pont ; pavois de bord ;
     flèches enflammées. Les volées produisent des `ShotEvent` (BV1) pour le rendu.
   - **Grappins et abordage** : un navire qui aborde lance les grappins quand les coques se
     touchent, se range bord à bord ; mêlée de pont où celui qui doit monter sur un pont plus
     haut est pénalisé (la cogue haute contre la galère basse), châteaux en défense, renforts
     par les chaînes (l'Écluse).
   - **Incendie et brûlots** : le feu croît avec le vent, gagne les navires amarrés, ronge coque
     et équipage ; les marins le combattent ; au-delà de 75 % l'équipage se jette à l'eau.
   - **Capture ou naufrage** : un équipage brisé amarré à l'ennemi amène ses couleurs (prise) ;
     libre, il fuit ; une coque détruite sombre, les hommes en armure se noient.
2. **Auto-résolution** (`naval::auto_resolve`) : mêmes formules (`naval::combat`) jouées par
   phases, comme N1 (ADR 0013) : volées (davantage pour la flotte au vent), brûlots, éperon,
   manches d'abordage appariées, feu, redditions, fuite du vaincu. Calibrée sur la bataille 3D
   par un test (`sim-battle/tests/nv1_naval.rs`).
3. **Campagne** (`sim-campaign::naval`) : pas de flotte-entité. Chaque faction a une réserve de
   navires par classe (`data/naval/fleets.json`) et une maîtrise de chaque mer (0-100). Une
   traversée (`Embark`) peut être interceptée par une faction en guerre qui arme des navires
   dans cette mer ; la bataille navale suit (3D ou automatique pour le joueur, automatique pour
   l'IA). Conséquences : pertes des régiments embarqués, navires coulés et pris (ils changent de
   réserve), maîtrise de la mer au vainqueur, blocus des ports ennemis d'une mer tenue.
4. **Rendu** : scène `game/scenes/naval/naval_battle.tscn` ; lecture seule de `NavalBattleSim`.

## Conséquences

- La ligne « pas de bataille navale » du document de conception est caduque.
- `GameData` charge `data/naval/` (absent : aucune flotte, jamais d'interception).
- L'état de campagne gagne un champ `naval` (valeur par défaut pour les sauvegardes anciennes,
  pas de changement de `STATE_VERSION`).
- Les traversées deviennent risquées en mer ennemie ; la maîtrise de la Manche compte.

## Complément NV2 (2026-09-25)

- **Abordage général** (`naval::ai`) : une flotte IA libre appelle l'abordage général quand la
  majorité de ses navires est à `assault_range_m` de sa cible et la majorité de l'ennemi à
  portée, depuis `assault_softening_s` ; avant, elle approche et tire en bloc au lieu d'aborder
  navire par navire. Les cibles sont réparties : les plus forts choisissent d'abord, au plus
  `boarders_per_target` abordeurs par navire, les navires masqués (seconde ligne) sont pénalisés.
  Événement `Assault` pour la chronique.
- **Lignes enchaînées** : formation défensive décrite par le scénario (`chains` : centre, cap,
  écart), les navires d'un groupe y sont rangés bord à bord. Les abordeurs qui prennent un
  navire enchaîné passent sur le suivant par-dessus la prise (`CHAIN_CROSSING_S`), et un navire
  enchaîné frais tombe sur l'ennemi amarré à son voisin. Les équipages enchaînés ne peuvent fuir :
  leurs pertes de moral aux volées et aux prises sont multipliées par `chain_morale` (0,3), ce
  qui fait de l'Écluse une mêlée générale (4 à 5 abordages simultanés, 6 à 8 navires anglais à
  l'abordage) au lieu d'une reddition sous les flèches. L'auto-résolution applique le même
  facteur ; accord auto/3D 25/26.
- **Noms des navires de campagne** : `data/naval/ship_names.json` (schéma
  `naval_ship_names`), par faction et par port d'attache ; l'escadre prend les noms des ports de
  la faction sur la mer du combat, les transports ceux du port de départ, puis les noms de la
  faction (décalés par la graine) ; « Nef n°3 » seulement une fois la liste épuisée.
- **Mer d'une traversée** : un port peut préciser sa mer (`Settlement::sea_zone` : Calais,
  Wissant, Boulogne, Douvres sur la Manche) ; `fleets.json::port_waters` nomme les eaux (« le pas
  de Calais »).
- **Rendu** : coques en bordé à clin procédural (`naval_hull.gdshader`, espace objet, sans
  texture externe ni triangle ajouté), châteaux peints patinés (`naval_paint.gdshader`),
  brûlures persistantes ; bandeau des navires compact sur deux lignes et défilant.
