# ECO-R1c — « L'impôt consenti » : IA et Angleterre

Spec : `docs/audits/2026-10-29-economie-politique.md` § 6 R1 et § 8 R1-c. Base : ADR 0813, `docs/wip/eco-r1ab.md`.
ADR du lot : `docs/decisions/0815-eco-r1c-ia-et-angleterre.md` (numéro à la fusion).

## État : TERMINÉ (30/10)
Fait :
- Formes `wool_custom` (assiette = provinces productrices de `res_wool`, flux = `trade::export_openness`) et `poll_tax` (1377), champs
  `yield_factor` / `export_good` (modèle, schéma, données).
- IA : honneur (monnaie forte, trêve promise, acceptation selon faisabilité / nécessité chiffrée), convocation sur prévision de guerre
  (`war_shortfall`), choix (assemblée, forme), palier selon le manque. Tests `tests/economy/eco_r1c_ai_england.rs`.
- Constat : au départ de 1337 l'embargo flamand de 1336 ferme la maltôte (Calais est français) ; elle s'ouvre à sa levée.

Mesures (sonde 3 graines, 400 tours, France armée / dépenses ; part des impôts exceptionnels) :
| essai | France armée | France exc. | Angleterre armée | Angleterre exc. |
|---|---|---|---|---|
| base R1ab (note eco-r1ab) | 17,2 / 20,4 / 22,1 | 37 / 34 / 45 | 19,5 / 16,8 / 15,4 | 44 / 46 / 46 |
| a1 : prévision + `campaign_below` 6, palier >= 5 saisons | 12,1 / 21,9 / 16,5 | 49 / 36 / 48 | 13,4 / 12,6 / 14,2 | 62 / 71 / 63 |
Lecture a1 : trop d'argent levé, thésaurisé (trésor moyen 80-200 k pour 30-80 k de revenu) ; la ponction d'épargne (`adm`) fait
41-68 % des dépenses, l'armée reste à 12-23 %.
- a2 : palier au plus près du manque (`target_seasons` 2), `campaign_below_seasons` 3, `data/ai/campaign.json` `reserve_seasons` 3 -> 2
  (le surplus au-delà de 2 saisons part en recrues) : en cours.

## Tableau final (sonde 3 graines, 400 tours ; armée = entretien + sièges / dépenses ; exc. = levées / (levées + recettes))
| essai | France armée | France exc. | Angleterre armée | Angleterre exc. | défauts / trésor < 0 |
|---|---|---|---|---|---|
| R1ab | 17,2 / 20,4 / 22,1 | 37 / 34 / 45 | 19,5 / 16,8 / 15,4 | 44 / 46 / 46 | 0 |
| a1 prévision + campagne | 12,1 / 21,9 / 16,5 | 49 / 36 / 48 | 13,4 / 12,6 / 14,2 | 62 / 71 / 63 | - |
| a2 palier 2 saisons, reserve 2 | 25,8 / 24,5 / 22,0 | 36 / 36 / 31 | 22,6 / 23,5 / 19,4 | 61 / 59 / 57 | Angl. 0 / 32 / 22 tours < 0 |
| a3 a2 + parts militaires 0,85/0,55, hoard 2 | 19,7 / 22,1 / 24,7 | 40 / 27 / 30 | 27,8 / 19,8 / 20,4 | 52 / 60 / 47 | sans gain, abandonné |
| **a4 (retenu)** a2 + ponction d'épargne 15/20/25 -> 5/8/10 % | 37,4 / 31,8 / 26,5 | 32 / 40 / 40 | 28,1 / 31,6 / 31,9 | 61 / 58 / 56 | défauts : France 2/0/0, Angl. 0/0/0 |
| a5 ponction 2/4/6 % | 27,0 / 47,2 / 22,4 | 30 / 28 / 25 | 43,8 / 37,4 / 24,4 | 50 / 67 / 42 | Angl. 12 défauts (graine 3) : rejeté |
Retenu a4. Levier décisif : la ponction d'épargne (`economy.json` `opulence_*`, que l'audit prévoyait de réduire à un filet).

## Écarts à la spec
- Part de l'armée 26-37 % (France), 28-32 % (Angleterre) : sous la bande 35-60 % sauf une graine. L'armée reste bornée par le recrutement
  (réserve, nombre de recrues par tour, ADR 0102) : levier de R2 (mobiliser, payer, licencier), pas de R1. Les parts militaires et
  `hoard_spending_turns` de l'IA n'y changent rien (a3).
- Part exceptionnelle France 32-40 % (bande 25-60 % ok), Angleterre 56-61 %.
- Angleterre IA : trésors élevés (150-420 k) en fin de série, signe d'un surplus de subsides non dépensé (R2).
- Tests ajustés pour l'équilibre : `economy_tuning_comes_from_data` (opulence 15 -> 5), `the_hoard_drain_is_progressive` (12 saisons
  perdent au moins 1 saison de revenu, au lieu de 3), `ai_convenes_before_a_campaign` (trésor 3 revenus au lieu de 5 : la prévision de
  guerre se resserre avec le palier à 2 saisons).
- Seed bots --quick : dérive d'équilibre 0. Pas de fixture d'auto-résolution touchée.
