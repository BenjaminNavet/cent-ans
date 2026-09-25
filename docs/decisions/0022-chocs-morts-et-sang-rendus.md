# 0022 — Chocs de cavalerie, morts, sang et démembrements : règles au cœur, rendu par instances

Date : 2026-09-25 (lot BV2, « bataille vivante » 2 ; complète les ADR 0006 et 0014)

## Contexte
Le joueur demande des chocs de cavalerie qui renversent et projettent les fantassins, des morts
variées, des cavaliers désarçonnés, du sang sur les figurines et des démembrements (backlog TW,
idées J). Contraintes : toute règle dans `core/` ; soldats rendus par `MultiMesh` skinnés (texture
d'os, ADR 0014), sans squelette ni corps physique par soldat ; plus de 10 000 soldats à tenir
(`--units=50`).

## Décision
- **Règles au cœur** (`sim-battle/src/impact.rs`) : un **événement d'impact par charge** (par
  paquet, pas par soldat) : sorte (choc, piques, pieux, charge brisée), point de contact, cap,
  poids de charge, **renversés**, **désarçonnés**, profondeur de pénétration, cohésion perdue.
  Tout est déterministe (aucun tirage) : poids = charge × lances × coin × chevaux ; part renversée
  selon l'exposition (tireurs 0,5, fantassins 0,35), l'armure et l'angle (flanc ×1,4, dos ×1,8),
  nulle en carré et contre des cavaliers ; au plus 1,5 homme par cavalier. Effet de règle : les
  renversés ne combattent pas pendant 3 s. Les piques (capacité schiltron) de front ou en carré
  arrêtent la charge (8 % des cavaliers, moral −10). La cohésion reste le choc existant (8/15) :
  un malus de moral supplémentaire déséquilibrait les batailles miroir de l'IA.
- **Cause des pertes** : chaque régiment garde `loss_cause` (flèche, carreau, boulet, pierre,
  mêlée, charge, pieux, piques, feu) et `loss_by` ; le pont les expose dans `get_units()` avec
  `get_impacts()`. Le rendu choisit la mort d'après la cause (`data/fx/battle_gore.json`).
- **Rendu par instances, sans physique** :
  - cadavres, renversés et cavaliers désarçonnés sont des instances en mode CUSTOM du shader
    skinné : `INSTANCE_CUSTOM` = (instant, clip, vitesse de projection, partie tranchée + sang).
    La **projection** est une parabole calculée dans le shader (vers −Z du modèle, la figurine
    tournée vers son tueur) ; les renversés regagnent leur place à la fin du clip `knockdown`,
    leur rang est vidé dans la formation (base nulle) le temps de la chute ;
  - le cavalier désarçonné (`c_fall`) garde son cheval, qui détale et disparaît (code 6 : os du
    cheval décalés dans le shader) ;
  - **démembrement** : masque par os (table `sever_bones` par rig) dans une **variante
    « cadavres » du shader** (`#define BV2_CORPSE`, `discard` sur le poids de la partie > 0,5,
    moignon rougi en deçà) — les soldats vivants gardent un shader sans `discard` ;
  - morceaux tranchés et gerbes de sang : deux tampons circulaires en `MultiMesh`, trajectoires
    balistiques et rotation calculées dans `battle_gore.gdshader` (pas de `RigidBody` : coût fixe,
    aucune simulation côté GDScript) ;
  - chevaux ralentis dans la masse ou arrêtés par les piques : **retard d'horloge d'animation**
    par régiment (le temps du matériau prend du retard, sans saut de phase), pas de règle ;
  - cadavres rangés par **cellules de terrain** de 80 m (maillage moyen < 60 m, lointain
    au-delà, masqués après 380 m), plafond global 14 000 puis remplacement.
- **Réglage « Sang »** `battle/blood` (0 désactivé, 1 modéré par défaut, 2 complet) partagé avec
  BV1 ; les démembrements n'existent qu'en « complet ».
- **Point d'accroche** : signal `BattleSoldiers.corpse_fallen(position, camp, famille, cause)`
  pour planter des traits dans les cadavres (BV1).

## Conséquences
- Une empreinte de test change (`b6.rs`, seed 3 : une perte de plus) ; les autres batailles de
  référence et l'IA sont inchangées.
- Le shader skinné porte deux variantes compilées (vivants, cadavres).
- Les renversés reviennent à leur rang par un court glissement ; une figurine tuée pendant sa
  chute peut laisser un double bref (cadavre à son rang + renversé), accepté.
- Les morceaux tranchés sont des primitives (sphère, capsules) teintées, pas des parties du
  maillage : lisibles de loin, sommaires en très gros plan.
