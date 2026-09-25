# ADR 0025 — Négociation à plusieurs clauses, buts de guerre et fatigue de guerre (lot DP1)

Date : 2026-09-25. Statut : accepté.

## Contexte

L'audit A2 (lot N2) relevait une carte figée : environ 7 changements de propriétaire de province
par partie de 200 tours, des guerres courtes qui finissent en statu quo, et une guerre
France-Angleterre présente 36 à 47 % du temps sur le siècle, pour une cible de 55 à 75 %. La
diplomatie (M5) ne connaissait que des propositions d'un seul objet (paix avec cessions et tribut,
alliance, vassalité, mariage), jugées par oui ou par non. L'écran était un panneau de 980 px
(audit A3 : champ « tribut positif ou négatif », barres sans nombre, carte diplomatique peu
lisible, légende coupée : défauts D1, C11 et U14).

## Décision

1. **Traités à plusieurs clauses** (`sim-campaign/src/negotiation.rs`) : `Article` = paix, trêve,
   alliance, accès militaire, accord commercial, mariage, tribut par saison, or, cession de
   province, cession de place (colonie), vassalité, libération d'un captif, otage. Chaque clause a
   un donneur (`Party::Proposer` ou `Party::Recipient`), sauf les clauses communes. Ordre
   `propose_treaty {target, articles}` ; `Proposal::Treaty` passe par les offres au joueur comme
   les autres propositions.
2. **Évaluation** (`evaluate_treaty`, pure) : valeur de chaque clause pour le destinataire (avec
   ses raisons), plus des considérations générales : attitude (÷ 3, ÷ 5 pour une paix), confiance
   (parjures, otages, mariages, traités en cours), menace (voisin bien plus puissant), honneur
   (alliés abandonnés par une paix séparée), fatigue de guerre, prétention à la couronne, buts
   de guerre non atteints, prudence. Le score donne une **chance d'acceptation**
   `100 / (1 + e^(-score / 8))`. La réponse est un **tirage déterministe** par (graine, tour,
   proposant, destinataire) : la même offre reçoit la même réponse pendant la saison, et le
   joueur voit la chance avant d'envoyer.
3. **Contre-proposition** (`counter_proposal`) : on complète l'offre avec ce que le proposant
   peut céder (captifs, provinces que l'ennemi occupe déjà, or, tribut) ou on retire ses
   exigences les plus coûteuses, jusqu'à 65 % de chance.
4. **Buts de guerre, score et fatigue** (N2) : à chaque nouvelle guerre, chaque camp vise jusqu'à
   deux provinces (revendiquées, puis frontalières) ; les tenir ajoute 12 points au score de
   guerre, les places secondaires occupées 3 points chacune (20 au plus). La **fatigue de guerre**
   (0-100) croît de 1 à 2 points par saison de guerre qui pèse (ennemi frontalier assez fort,
   score négatif ou terres occupées), décroît de 4 en paix, ajoute du mécontentement
   (fatigue ÷ 5 par province) et pousse à la paix (fatigue ÷ 4 dans l'évaluation).
5. **Paix de l'IA** (`plan_peace`) : pas de traité avant 20 saisons de guerre (sauf royaume
   acculé) ; le vainqueur (score ≥ 20) exige les provinces qu'il tient, buts de guerre d'abord,
   tant que la chance reste ≥ 60 % ; un royaume épuisé (fatigue ≥ 70) ou écrasé (score ≤ -50)
   achète la paix par la contre-proposition. Trêve de 2 ans après une paix (Malestroit,
   Bordeaux). Un prétendant au trône part en guerre à crédit (une saison d'entretien en caisse
   au lieu de deux) et tolère un front secondaire deux fois plus lourd (Édouard III et les
   Écossais).
6. **Accords** : un accord commercial vaut +2 % de revenu (+8 % au plus), l'accès militaire
   ravitaille nos armées sur les terres de l'autre (`is_friendly_territory`), le tribut est
   versé chaque saison, l'otage est tenu captif 40 saisons ; une guerre rompt tout, et l'otage
   trahi laisse un malus de confiance.
7. **IA des accords** (`ai/src/diplomacy_eval.rs`, une ligne dans `campaign.rs`) : accords
   commerciaux avec des voisins ou alliés bien disposés, accès militaire réciproque avec les
   voisins neutres d'un ennemi.
8. Réglages dans `data/ai/diplomacy.json` § `negotiation` (schéma à jour) ; `enabled: false`
   (défaut sans le fichier) garde la paix de G5.
9. **Écran plein** (`game/scripts/ui/diplomacy_panel.gd`, même classe `DiplomacyPanel`, donc la
   même place dans la pile de panneaux d'UI2) : factions (blason, souverain, relation, attitude,
   raisons en infobulle), carte des relations (minicarte recolorée, clic = faction), fiche,
   onglets Négociation / Guerre / Traités. La carte diplomatique 3D (touche N) renforce la teinte
   des provinces et affiche une légende permanente en bas au centre ; le panneau de province se
   ferme à l'entrée dans le mode (C11, U14). `map_ui.gd` n'est pas modifié.

## Mesures (sonde)

| Mesure | Avant (main + G1) | Après DP1 |
|---|---|---|
| `century_probe` 5 × 464 : guerre FR-EN | 36 % (30-42) | **61 % (43-68), 4/5 graines dans 55-75 %** |
| `century_probe` : 4 majeures vivantes en 1400 | 5/5 | 5/5 |
| `balance_probe campaign 200 1-8` : guerre FR-EN | 33 % | 54 % |
| Changements de propriétaire (200 tours) | 6,9 | 13,9 |
| Mécontentement moyen final | 8,3 | 12,0 |
| Révoltes par partie | 11,5 | 21,1 |
| Paix signées / guerres déclarées par partie | 77 / 88 | 104 / 119 |

Traités observés (`dp1_probe`, graine 2) : Calais et le Boulonnais, le Périgord, le Quercy, la
Normandie cédés à l'Angleterre ; la France rachète la paix contre or et tribut.

## Conséquences

- Les propositions à un seul objet (M5) restent valides ; leur évaluation n'a pas changé, sauf
  le score de guerre, qui compte désormais les buts de guerre et les places occupées.
- Le hasard ne joue que sur la réponse, jamais sur l'évaluation : une sauvegarde rejouée donne
  les mêmes traités.
- Le commerce de C5 (`Proposal::TradeAgreement`, `integration/tw`) n'est pas sur main : l'accord
  commercial de DP1 est autonome (`DiplomaticLedger::trade_agreements`). À la fusion de C5, il
  faudra faire de la clause `trade_agreement` l'accord de C5.
