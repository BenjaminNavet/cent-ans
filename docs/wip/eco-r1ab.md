# ECO-R1ab — « L'impôt consenti » (boucle juge 2)

Spec : `docs/audits/2026-10-29-economie-politique.md` § 6 R1 et § 8 (R1-a, R1-b). R1-c (IA, Angleterre, maltôte) et R1-d
(interface) viennent après : l'API publique de `sim-campaign::assembly` est prévue pour eux. ADR : `docs/decisions/XXXX-eco-r1-impot-consenti.md`
(à numéroter à la fusion).

## État : terminé (R1-a, R1-b), prêt à fusionner
- R1-a : données (`data/rules/assemblies.json`, schéma, ligne `test_schemas.py`), `data-model::AssemblyRules`, état
  `CampaignState::assemblies`, ordres `ConveneAssembly` / `AnswerAssembly`, `AssemblyError`, invariant `assembly_refs`.
- R1-b : `sim-campaign/src/assembly.rs` (vote, perception par saison, concessions, promesses, humeur, permanence,
  grogne locale, causes, IA minimale). Tests : `tests/economy/eco_r1_assembly.rs` (18).
- Hors spec mais nécessaire : l'IA planifie sur la part de cour de l'administration
  (`CampaignState::administration_share_for`), sans la ponction d'épargne : un subside remplit le trésor, la ponction
  faisait voir un déficit à l'IA, qui licenciait ses troupes (unités de la France IA : 3 au lieu de 10).
- Vérifié : `cargo test --workspace` (seuls échecs : les 3 tests rouges de main), clippy `-D warnings` core et client, passe rapide
  seed bots (0 constat de l'ère R1 après réglage des bandes), `uv run --project tools pytest tools/tests/test_schemas.py`.

## API pour R1-c / R1-d
- `assembly::overview(state, data, faction) -> Vec<AssemblyView>` (humeur, convocable ?, nécessités, formes, session ouverte, promesses).
- `assembly::forecast(state, data, faction, assembly, necessity, ask, form, permanent) -> Vote` (accord, parties, issue, conditions, montant).
- `assembly::can_convene`, `available_necessities`, `available_forms`, `covered_provinces`, `mood_of`, `province_unrest`.
- `assembly::ai_assembly_orders(cache, data, faction)` : minimal (convoque si trésor prévisible < N saisons, nécessité, aucun subside en
  cours, prévision différente d'un refus ; répond selon un plafond de coût). R1-c : honneur, maltôte, prévision de guerre.
- Causes : `assembly_refused`, `promise_broken`, `subsidy_granted` (builders dans `cause_builders.rs`, textes `data/ui/causes.fr.json`).
- `EventKind::Assembly` ; `FinanceError::AssemblyRequired` quand une maison à assemblée tente `LevyExtraordinary`.

## Mesures (sonde `eco_series --seeds 1,2,3 --factions fac_france,fac_england`, 400 premiers tours, IA aux commandes)
| | avant | après |
|---|---|---|
| France : impôts exceptionnels / recettes | 4,6 / 2,2 / 2,9 % | 37,0 / 33,7 / 44,5 % |
| France : armée (entretien + sièges) / dépenses | 11,1 / 20,4 / 14,6 % | 17,2 / 20,4 / 22,1 % |
| Angleterre : exceptionnel | 2,2 / 2,5 / 2,5 % | 44,4 / 45,7 / 45,8 % |
| Angleterre : armée | 13,7 / 13,7 / 13,1 % | 19,5 / 16,8 / 15,4 % |
| France : sessions refusées + à ≥ 2 concessions, en 40 ans (160 tours) | sans objet | 2 / 3 / 2 refusées, 0 / 4 / 4 à conditions lourdes |
| France : sessions convoquées en 400 tours (accordées) | — | 89 (80) / 71 (63) / 76 (72) |
| France : trésor négatif (saisons) / banqueroutes | 11 / 0 / 19, 2 / 0 / 6 | 0 / 0 / 0, 0 / 0 / 0 |
Avant (première version, IA qui licenciait) : part de l'armée 3-6 % ; le correctif de planification de l'IA l'a ramenée au-dessus de l'état d'avant.

## Écarts à la spec
- Part de l'armée 17-22 % pour une cible de 35-60 % : non atteinte. Cause mesurée : l'IA dépense peu des subsides (réserve de
  recrutement, ADR 0102) et la ponction d'épargne (`opulence`, ADR 0749/0788) gonfle l'administration (jusqu'à 25-45 k par saison)
  quand le trésor dort. Pistes : R1-c (convoquer sur prévision de guerre) et R2 (mobiliser, payer, licencier), et réduire la
  ponction d'épargne (non fait : toucher `economy.json` déplace `starting_fit` et plusieurs tests).
- Ordinaire « ≈ −30 % » non appliqué : `tax_efficiency` 0,056 met la France IA de départ en austérité (deux tests d'`ai` tombent,
  l'IA licencie). `economy.json` est inchangé.
- Armées plus grosses : `men_under_arms` de la France IA atteint 5 000-13 000 en fin de siècle (graines 2 et 3), à surveiller par R2.
- Les boutons d'impôt de la fenêtre Finances restent affichés pour une maison à assemblée et sont refusés (R1-d).
- Passe longue des seed bots (fin de lot) : l'IA du jeu (mirror) ne dérive que sur `ai_refused_orders` (593 pour une bande à 400, tous les bots
  dérivent dans le même sens, 1 ordre refusé sur 512 touche une assemblée) et `men_under_arms_peak` (4 890, bande relevée à 9 000) ;
  les autres dérives (révoltes, provinces tenues, banqueroutes des bots naïfs) concernent des bots qui ne convoquent aucune assemblée.
- Bandes des seed bots : `army_spend_share` [0,04 ; 0,45] (rapide) / [0,04 ; 0,7] (longue), `extraordinary_income_share` [0 ; 0,65].

## Reste (suites)
- R1-c : maltôte de la laine, capitation, honneur des promesses, convocation sur prévision, `bankruptcies_per_game`.
- R1-d : onglet « États et Parlement », textes `data/ui/assembly_panel.json`, codex, conseiller.
- Retirer ou réduire la ponction d'épargne après mesure.
