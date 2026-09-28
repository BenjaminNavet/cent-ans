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
Les défenseurs à moins de 45 m du centre de la place, une fois la ville ouverte (une brèche ou la
porte tombée), perdent moins de moral à leurs pertes (× 0,85) et aux déroutes voisines (× 0,8), et
en regagnent 0,08/s même au contact (jusqu'au plafond).

### IA
- **Garnison** : repli sur la place dès la **première brèche** (plus seulement à la chute de la
  porte ; message « La muraille est rompue : la garnison se replie sur la place ! »), sans laisser de
  régiment tenir les ouvertures (`max_blockers` = 0) ; repliés, les défenseurs ne chargent que les
  assaillants à moins de 30 m du centre (ou à 40 m d'eux) au lieu de 250 m.
- **Assaillant** : les régiments de mêlée entrés dans la ville se regroupent à 35 m à l'intérieur de
  la brèche et ne marchent sur la place qu'une fois 60 % d'entre eux dedans (ou dès que l'un d'eux
  est à moins de 70 m de la place) ; les tireurs quittent le duel avec le chemin de ronde et
  convergent vers la place dès que de la mêlée amie est dedans.

### Données
`data/rules/siege_capture.json` (schéma `data/schemas/siege_capture_rules.schema.json`, pytest
`tools/tests/test_siege_capture_schema.py`) : points, supériorité, seuil d'alerte, dernier carré,
repli, assaut groupé. `battle_alerts.json` gagne `square_threatened`. La constante
`siege::HOLD_TO_WIN` disparaît (la durée vient des données), `siege_fx::BLOCKERS_PER_OPENING`
aussi (`fall_back.blockers_per_opening`).

### Pont et rendu
- `get_siege().points[{kind, x, z, radius, progress, hold_s, share, status, attackers, defenders}]`
  (points prévus dès le déploiement), `hold_to_win` lu dans les règles.
- `game/scripts/battle/siege_capture_points.gd` (`SiegeCapturePoints`) : drapeau planté au sol aux
  couleurs du camp qui tient le point, cercle au sol du rayon du point qui vire à la couleur de
  l'assaillant avec la prise, barre de capture enluminée (style des barres SB) « Place du marché :
  24/60 s », visible dès que la prise est entamée ou disputée. `BattleSiege` la crée et la met à jour.
- Colonne d'alertes : libellé « La place est menacée », glyphe vectoriel (drapeau).

## Mesures

MESURES

## Conséquences

CONSEQUENCES
