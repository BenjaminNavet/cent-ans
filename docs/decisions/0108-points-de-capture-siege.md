# 0108 — Points de capture en siège et rééquilibrage des assauts (lot TW2 T4)

Date : 2026-09-28. Statut : accepté. Spec : `docs/design/2026-09-28-tw2-mecaniques-total-war.md` § T4.
Suite de l'ADR 0107 (lot SB, rythme de destruction des murs).

## Contexte

Le lot SB a raccourci la destruction des murs (brèche vers 70 s au lieu de 275 s sur le banc BR3).
Sur `br3_assault_probe` (11 régiments dont trébuchet, mangonneau et beffroi contre 6 de garnison,
niveau 2, murs entamés à 40 %), l'assaillant est passé de 3/10, 3/10, 2/10 prises (générique, Paris,
Rouen) à 10/10 partout. La trace montrait pourquoi : la garnison, éparpillée aux brèches
(2 régiments par ouverture), se débandait par morceaux près des murs, souvent presque intacte, sans
jamais défendre la place. La victoire par la place existait (60 s sans aucun défenseur dessus) mais
servait peu : les batailles finissaient par la déroute de la garnison.

Consigne : revenir à 4-7/10 sans rallonger la destruction des murs, par des mécaniques Total War
(défense de la place centrale), `sg3_assault_probe` (armées de campagne) restant proche de l'actuel.

## Décision

### Points de capture (cœur `sim-battle/src/capture.rs`, pas `sim/capture.rs`)
- Deux points par siège, construits au premier pas de la bataille (`SiegeWorks::points`, champ
  `serde(default)`, pas de changement de `STATE_VERSION`) : la **place du marché** (centre de la ville,
  point de victoire) et la **porte** (22 m à l'intérieur de la porte).
- Règle TW : un point progresse d'une seconde par seconde tant que l'assaillant y a **plus d'hommes
  valides** que le défenseur (× `superiority` = 1) ; sinon la progression recule (0,5 s/s si
  personne n'y domine, 2 s/s si le défenseur y domine). Comptent les régiments valides au sol (ni
  sur le chemin de ronde, ni à l'échelle, ni machines).
- Place tenue 60 s : la ville est prise (`BattleEnd::SquareHeld`, inchangé). `SiegeWorks::hold_time`
  reste le miroir de la progression de la place (HUD).
- Porte tenue 30 s : elle s'ouvre (PV à 0, compte comme une ouverture) et les tours à moins de
  90 m de la porte se taisent.
- Alerte typée `square_threatened` (« La place est menacée », importance 95) quand la place atteint
  25 % de sa prise ; réarmée quand la progression retombe à 0.

### Dernier carré
Les défenseurs à moins de 35 m du centre (la place elle-même) de la place, une fois la ville ouverte (une brèche ou la
porte tombée), perdent moins de moral à leurs pertes (× 0,85) et aux déroutes voisines (× 0,8), et
en regagnent 0,08/s même au contact (jusqu'au plafond).

### IA
- **Garnison** : repli sur la place dès la **première brèche** (plus seulement à la chute de la
  porte ; message « La muraille est rompue : la garnison se replie sur la place ! »). Tant que la
  porte enfoncée est la seule entrée, deux régiments tiennent le corps de porte (comme avant, SG1) ;
  dès qu'une muraille est rompue, aucun régiment ne reste aux ouvertures (`breach_blockers` = 0) ; repliés, les défenseurs ne chargent que les
  assaillants à moins de 30 m du centre (ou à 40 m d'eux) au lieu de 250 m.
- **Assaillant** : les régiments de mêlée entrés dans la ville se regroupent à 35 m à l'intérieur de
  la brèche et ne marchent sur la place qu'une fois 60 % d'entre eux dedans (ou dès que l'un d'eux
  est à moins de 70 m de la place) : plus de régiment jeté seul contre le dernier carré. Essayé puis
  retiré : faire converger aussi les tireurs vers la place (effet nul sur br3, mais Rouen passait de
  7/10 à 10/10 sur sg3).

### Données
`data/rules/siege_capture.json` (schéma `data/schemas/siege_capture_rules.schema.json`, pytest
`tools/tests/test_siege_capture_schema.py`) : points, supériorité, seuil d'alerte, dernier carré,
repli (`gate_blockers`, `breach_blockers`, `max_blockers`), assaut groupé. `battle_alerts.json` gagne `square_threatened`. La constante
`siege::HOLD_TO_WIN` disparaît (la durée vient des données), `siege_fx::BLOCKERS_PER_OPENING`
aussi (`fall_back.gate_blockers`). Le test F5d (escalade sans brèche, 2-5/6 prises) a imposé de garder les gardes de la porte tant qu'elle est la seule entrée : sans eux, 6/6.

### Pont et rendu
- `get_siege().points[{kind, x, z, radius, progress, hold_s, share, status, attackers, defenders}]`
  (points prévus dès le déploiement), `hold_to_win` lu dans les règles.
- `game/scripts/battle/siege_capture_points.gd` (`SiegeCapturePoints`) : drapeau planté au sol aux
  couleurs du camp qui tient le point, cercle au sol du rayon du point qui vire à la couleur de
  l'assaillant avec la prise, barre de capture enluminée (style des barres SB) « Place du marché :
  24/60 s », visible dès que la prise est entamée ou disputée. `BattleSiege` la crée et la met à jour.
- Colonne d'alertes : libellé « La place est menacée », glyphe vectoriel (drapeau).

## Mesures

Sondes : `cargo test -p sim-battle --test br3_assault_probe -- --ignored --nocapture probe_town_assaults`
(la colonne « Fins » est nouvelle ; `BR3_DEF_ROUTS=1` et `BR3_WATCH=1` tracent la garnison) et
`cargo test -p sim-campaign --test sg3_assault_probe -- --ignored --nocapture probe_landmark_assaults`.
Durée de destruction des murs inchangée (données `siege_works.json` non touchées).

### `br3_assault_probe` (victoires de l'assaillant, durée médiane)

| Ville | Avant SB | SB (avant T4) | T4, 10 graines | T4, 20 graines |
|---|---|---|---|---|
| générique | 3/10 (391 s) | 10/10 (201 s) | 7/10 (294 s) | 14/20 |
| Paris | 3/10 | 10/10 (242 s) | 2/10 (289 s) | 4/20 |
| Rouen | 2/10 | 10/10 (328 s) | 3/10 (365 s) | 8/20 |
| **ensemble** | 8/30 | 30/30 | 12/30 | 26/60 (4,3/10) |

Pertes moyennes (assaillant / garnison), générique : 30 / 116 avec SB, 86 / 173 avec T4 : on se bat
désormais vraiment pour la place.

Réglages essayés (10 graines, générique / Paris / Rouen) :

| Réglage | Résultat |
|---|---|
| repli à la brèche, 2 régiments par ouverture, dernier carré fort (0,6 / 0,25 / 0,5, 70 m) | 10 / 8 / 6 |
| aucun régiment aux ouvertures, dernier carré fort | 2 / 0 / 0 |
| 2 régiments à la porte même après une brèche | 9 / 8 / 10 |
| 1 régiment aux ouvertures, dernier carré fort | 8 / 8 / 3 |
| aucun aux ouvertures, sans dernier carré | 7 / 2 / 5 |
| + assaut groupé (60 %) | 8 / 3 / 4 |
| retenu : + dernier carré léger (35 m, 0,85 / 0,08 / 0,8), charge à 30 m | 7 / 2 / 3 |

Le régiment laissé seul à une brèche est le point de bascule : il se débande, et la contagion des
déroutes (tout déroutant à moins de 120 m compte en siège) emporte la garnison entière sans
qu'un assaillant soit entré. Repliée d'un bloc sur la place, la garnison tient.

### `sg3_assault_probe` (armées de campagne, 10 graines)

| Ville | Victoires SB → T4 | Porte (médiane) SB → T4 | Durée médiane SB → T4 |
|---|---|---|---|
| Paris | 10/10 → 10/10 | 202 → 202 s | 291 → 271 s |
| Avignon | 10/10 → 10/10 | 202 → 202 s | 237 → 236 s |
| Bruges | 10/10 → 10/10 | 172 → 172 s | 178 → 178 s |
| Calais | 10/10 → 10/10 | 205 → 205 s | 211 → 211 s |
| Rouen | 7/10 → 7/10 | 286 → 274 s | 347 → 347 s |

(Rouen : la porte est parfois ouverte par sa prise — des assaillants entrés par l'échelle tiennent
le point de la porte.)

## Conséquences

- **Écart à la cible** : l'ensemble du banc revient dans la plage (4,3/10 sur 60 parties), le
  générique (7/10) et Rouen (4/10) aussi, mais **Paris reste bas (2/10)** : l'assaillant y arrive à
  la place par des rues étroites, entamé par les maisons en feu (chaleur, S2) et les arbalétriers
  repliés, et se brise à 40 m de la place. Aucun réglage essayé ne remonte Paris sans pousser le
  générique au-delà de 7/10 ; piste : chemins de l'IA qui évitent les incendies, ou tireurs de
  l'assaillant qui entrent en ville sans faire basculer sg3.
- Les batailles de siège finissent encore surtout par la déroute de la garnison ; la victoire par
  la place (« square_held ») concerne 10 à 40 % des prises.
- Le joueur défenseur n'a pas l'IA de repli : il doit ramener lui-même ses troupes sur la place,
  où le dernier carré joue pour lui aussi.
- L'alerte de la colonne n'a pas encore d'icône à l'encre DA5 (glyphe vectoriel de repli).
