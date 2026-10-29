# ADR 0788 — Score de règne : des actes, pas l'attente ; épargne dormante nette des dettes

Date : 2026-10-24. Statut : accepté (lot GAMEPLAY2) ; § 1 remplacé par l’ADR XXXX (renommée). Amende les ADR 0784/0785 (acte, score) et 0749 (ponction du trésor dormant).
Source : juge game design it. 4 (`out/juge/iter-04-design.md`, constats C1, C2, C4).

## Contexte

- **C1.** Au tour 2, sans une action, 4 objectifs sur 7 de l'acte I de la France étaient « remplis » (lignée, domaine tenu, dette
  nulle, alliés déjà 5/4) : score d'acte 35. Le score était la part pondérée des objectifs remplis, et la plupart sont des états
  de départ. Mesure (sonde `reign_probe`, IA complète des rivaux) : une France passive ne gagne jamais (elle s'effondre :
  défaite en 1403-1418), mais une France « IA » qui bâtit sans viser un objectif atteignait 40 à 73 (victoire à 73 sur la graine
  1341) sans conquête. Le « 66 » du test d'ACTES venait de l'IA minimale des rivaux (`end_turn`), pas du jeu réel.
- **C2.** La ponction de l'épargne dormante (ADR 0749, `EconomyRules::opulence`) portait sur le trésor brut : un emprunt de
  31 000 livres faisait passer la charge « Cour, administration » de 4 677 à 8 872, soit +13 % de la somme empruntée par saison,
  en plus des 4,5 % d'intérêt. Emprunter était absurde et l'infobulle ne le disait pas.
- **C4.** L'arc n'était ouvert qu'au premier `end_turn` : au tour 0 le panneau des objectifs montrait encore la règle abandonnée
  des 40 saisons.

## Décision

1. **Acquis et gains.** À l'ouverture de chaque acte, l'état de chaque objectif est mémorisé (`ActsState::start_permille`).
   Un objectif **déjà rempli** à ce moment est un *acquis* : il ne rapporte aucun point, il garde le score. Les autres sont
   des *objectifs à gagner* : rempli = tout son poids, sinon le quart (`PARTIAL_CREDIT`) de l'avancement **depuis l'état de
   départ** (plus de crédit pour une progression que l'inaction a faite). Score d'acte =
   `base × (1 − g + g × garde) + pressions`, avec `base` la part pondérée des objectifs à gagner (100 s'il n'y en a plus), `garde`
   la part pondérée des acquis encore tenus, et `g = hold_guard_percent` (35, `data/rules/acts.json`, 0 = ancien comportement).
   Aucun objectif n'est à marquer en données : la règle est générale. Les acquis s'affichent « — acquis à conserver »
   (`texts.hold_suffix`).
2. **Seuil inchangé** (72, couronné) : la sonde montre que le seuil est atteignable par qui conquiert les objectifs territoriaux
   et hors de portée de qui attend (voir mesures).
3. **Épargne nette.** `administration_upkeep_for` applique `opulence` au trésor *moins le principal dû* : l'argent emprunté
   n'est pas l'épargne de la maison. Emprunter ne change plus la charge d'administration (test
   `borrowing_does_not_raise_the_administration_charge`). Les infobulles (`tooltips.json`, `finance_panel.json`,
   encyclopédie) le disent.
4. **L'arc s'ouvre à la création de la campagne** (`acts::start`), donc dès le tour 0.

## Mesures (France, sonde `core/crates/seed-bots/examples/reign_probe.rs`, IA complète des rivaux)

Trois profils : *passif* (aucun ordre), *IA* (l'IA stratégique joue la maison : bâtit, recrute, s'allie, ne vise pas les objectifs),
*décisif* (IA + une campagne décisive par acte : les provinces des objectifs territoriaux passent à la maison quatre ans après
l'ouverture de l'acte, simulant les conquêtes d'un bon joueur).

| profil | graine | avant (score, issue) | après (score, issue) |
|---|---|---|---|
| passif | 1 | 28, défaite 1403 | 5, défaite 1403 |
| passif | 1341 | 30, défaite 1418 | 15, défaite 1418 |
| passif | 2 | 33, défaite 1451 | 13, défaite 1451 |
| IA | 1 | 40, clos | 15, clos |
| IA | 1341 | 73, victoire | 68, clos |
| IA | 2 | 49, clos | 28, clos |
| décisif | 1 | n/m | 67, clos |
| décisif | 1341 | n/m | 97, victoire |
| décisif | 2 | n/m | 76, défaite par ruine en 1417 (score du règne jusque-là) |

Lecture : passif ≤ 15 (cible du juge : ≤ 45), IA qui ne vise pas les objectifs 15-68 (sous le seuil de 72), décisif 67-97 (le
seuil est atteignable par qui conquiert). L'avant « décisif » n'a pas été mesuré (profil créé avec ce lot).

Score à l'ouverture de l'acte I (tour 0-2), France inactive : 35 avant, 0 après (test
`the_arc_starts_in_the_first_act_with_live_objectives`).

## Conséquences

- Les tests simulés qui lisaient le score (`acts.rs`) sont adaptés ; `f7_events` (graine 1341) et `ev_history` ne dépendent pas
  du score de règne mais de l'économie : la correction de l'épargne ne touche que les maisons endettées et en trésor positif
  (l'opulence de l'IA aussi, via la même fonction), elle n'a déplacé aucun de ces tests (aucun test déplacé : voir le rapport de lot).
- Saves : `start_permille` est `serde(default)` ; une sauvegarde d'avant compte tout depuis zéro (ancien comportement), sans
  migration.
- Limite : un acquis perdu puis regagné dans l'acte ne rapporte rien ; la progression des objectifs non linéaires (dette) est
  une approximation du ratio.
