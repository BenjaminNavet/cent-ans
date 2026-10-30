# ECO-R1c — « L'impôt consenti » : IA et Angleterre

Spec : `docs/audits/2026-10-29-economie-politique.md` § 6 R1 et § 8 R1-c. Base : ADR 0813, `docs/wip/eco-r1ab.md`.
ADR du lot : `docs/decisions/XXXX-eco-r1c-ia-et-angleterre.md` (numéro à la fusion).

## État : règles et tests en place, équilibre en réglage
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

## Suite
- Mesurer a2, régler, tests workspace, pytest, seed bots `--quick`, ADR, rapport.
