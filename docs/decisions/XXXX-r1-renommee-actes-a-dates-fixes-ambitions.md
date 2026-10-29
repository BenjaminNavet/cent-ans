# XXXX — R1 : renommée accumulée, actes à dates fixes, ambitions

Statut : accepté (numéro attribué à la fusion). Remplace 0785 et 0788 § 1 ; amende 0784 ; revient sur le seuil de
victoire de 0793.

## Contexte

L'audit `docs/audits/2026-10-29-campagne.md` (§ 4 R1, § 6) mesure, avec la sonde de chronique, un score de règne figé
pendant des décennies : l'acte se clôt tôt par la paix ou l'effondrement, un objectif rempli ne rapporte qu'à la clôture,
un acte sans rien à gagner vaut 100 (`base = 100`), et la politique d'acte n'est qu'une pondération sans effet sur la
simulation. Le joueur n'a pas de raison de jouer chaque saison jusqu'en 1453.

## Décision

1. **Actes à dates fixes.** Un acte se clôt avec son année (1360, 1415, 1453) et seulement alors. La paix durable et
   l'effondrement du domaine ne ferment plus un acte (`early_end` supprimé). La ruine de la maison reste une défaite
   (0784 § 8, inchangé).
2. **Renommée de maison** (`acts.json` › `renown`, état `ActsState::renown`, journal borné). Chaque saison, des faits
   ajoutent ou retirent de la renommée à l'acte en cours, chacun plafonné par saison : bataille gagnée/perdue, place
   prise/perdue, saison de chevauchée, rançon touchée/payée, trêve imposée (traité conclu avec un score de guerre
   ≥ `truce_min_war_score`). La renommée des faits de l'acte en cours s'use lentement (`decay_permille_per_season`).
3. **Objectifs.** Un objectif rapporte sa valeur (poids × `objective_met` %) à son propre acte **la saison où il est
   rempli**, et la rend s'il est reperdu. Un objectif déjà rempli au début de la campagne est un acquis : il ne rapporte
   rien, sa perte coûte autant (à partir de l'ouverture de son acte). Un objectif d'un acte futur rempli tôt est crédité
   tôt. Les pressions ajoutent ou retirent leurs points tant qu'elles tiennent pendant leur acte. Plus de `base = 100`.
4. **Scores.** Score d'acte = renommée de l'acte / `renown_target` (affiché borné 0-100) ; figé à la clôture. Score de
   règne = moyenne pondérée des trois actes (poids de 0784), avec un score d'acte **signé** (-100 à 100) : un acte dont
   la renommée est négative pèse sur le règne, sinon le score se figerait au plancher d'un acte malheureux. Le total
   est borné 0-100 et bouge jusqu'en 1453. La victoire n'est jugée qu'à la fin
   (1453, ou fin de campagne courte) ; aucune victoire avant.
5. **Ambitions** (remplacent les politiques, 0785). Quatre par acte et par maison écrite (France, Angleterre, Bourgogne ;
   un jeu `default` pour les autres), deux à tenir (`ambitions_to_choose`). Une ambition a des **effets** (vocabulaire
   commun des modificateurs, fusionnés dans `research::faction_tech_effects` comme une technologie, pour la maison du
   joueur seulement) et un `renown_bonus_percent` sur les sources et catégories d'objectifs qu'elle nomme. Le schéma
   refuse une ambition sans effet. Une troisième remplace la plus ancienne pendant `ambition_choice_seasons` saisons
   après l'ouverture de l'acte, puis le choix se fige.
6. **Seuil de victoire** : `victory_min_score` = médiane du profil IA (sonde, 3 graines × 3 maisons, 1453) + 15 ; voir
   « Mesures ». Revient sur le seuil 50 de 0793.
7. **Sauvegarde** : `STATE_VERSION` 12 ; migration v11→v12 (la politique devient la première ambition, les compteurs de fin
   précoce disparaissent, la renommée repart de la saison suivante).

## Mesures (sonde `campaign_timeline`, 470 tours, graines 1-3)

Profil IA : France, Angleterre et Bourgogne × 3 graines (9 parties). Profil passif : France × 3. « Avant » = sonde et
données à 95b504043 ; « après » = cette refonte (cibles 80/120/120, seuil 43).

| Mesure | Avant | Après |
|---|---|---|
| `score_frozen_years`, IA (médiane ; max) | 63 ; 90 | 0 ; 11 |
| Ouverture de l'acte II (IA) | 1349-1361 | 1361 (9/9) |
| Ouverture de l'acte III (IA) | 1364-1416 | 1416 (9/9) |
| Score de règne final, passif | 16, 16, 17 | 0, 0, 0 |
| Score de règne final, IA (médiane ; étendue) | 15 ; 1-53 | 28 ; 3-73 |
| Victoires IA avant 1453 | 0 | 0 |
| Victoires IA en 1454 (au seuil) | 1/9 (seuil 50) | 3/9 (seuil 43) |

- **Seuil** : médiane IA 28 + 15 = **43**.
- **Écarts.**
  - Le passif reste au plancher 0, si bien que `score_frozen_years` y est grand : le score ne descend pas sous 0.
  - Une partie IA (France, graine 2) reste figée 11 ans. La maison n'a alors plus d'armée et vit en paix : rien ne se
    passe, ce qui relève de l'IA (R5) et non de la règle.
- **Ambitions** (test `seed-bots/tests/ambitions.rs`) : chacune des 36 ambitions écrites déplace d'au moins 10 % une
  grandeur de la sonde sur 40 tours (trésor, prestige, effectifs, provinces). Sept ambitions trop faibles, des copies
  des anciennes pondérations, ont été renforcées.

## Conséquences

- Le client garde ses noms de champs (`policies`, `choose_policy`) ; il devra afficher la renommée (`ActView::renown`,
  `house_renown`, `renown_recent`), deux ambitions à choisir et leurs effets.
- Le lot TUTO affiche un `prestige` de faction provisoire ; à terme il lit `CampaignState::house_renown`.
- 0785 est remplacé ; 0788 § 1 (score par actes, acquis) est remplacé ; 0784 est amendé (§ fin d'acte, politiques).
