# ECO-R1ab — « L'impôt consenti » (boucle juge 2)

Spec : `docs/audits/2026-10-29-economie-politique.md` § 6 R1 et § 8 (R1-a, R1-b). R1-c (IA, Angleterre, maltôte) et R1-d
(interface) viennent après : l'API publique de `sim-campaign::assembly` est prévue pour eux. ADR : `docs/decisions/XXXX-eco-r1-impot-consenti.md`.

## État
- [x] R1-a : données (`data/rules/assemblies.json`, schéma, ligne `test_schemas.py`), `data-model::AssemblyRules`, état
  `CampaignState::assemblies`, ordres `ConveneAssembly` / `AnswerAssembly`, `AssemblyError`, invariant `assembly_refs`.
- [x] R1-b : `sim-campaign/src/assembly.rs` (vote, perception par saison, concessions, promesses, humeur, permanence,
  grogne locale, causes, IA minimale). Tests : `tests/economy/eco_r1_assembly.rs` (18).
- [ ] Équilibre mesuré à la sonde séculaire (en cours), seed bots rapide, clippy, client.

## API pour R1-c / R1-d
- `assembly::overview(state, data, faction) -> Vec<AssemblyView>` (humeur, convocable ?, nécessités, formes, session ouverte, promesses).
- `assembly::forecast(state, data, faction, assembly, necessity, ask, form, permanent) -> Vote` (accord, parties, issue, conditions, montant).
- `assembly::can_convene`, `available_necessities`, `available_forms`, `covered_provinces`, `mood_of`, `province_unrest`.
- `assembly::ai_assembly_orders(cache, data, faction)` : minimal (convoque si trésor prévisible < N saisons et nécessité ; répond selon un plafond de coût). R1-c : honneur, maltôte.
- Causes : `assembly_refused`, `promise_broken`, `subsidy_granted` (builders dans `cause_builders.rs`, textes `data/ui/causes.fr.json`).
- `EventKind::Assembly`.

## Mesure « avant » (sonde `eco_series --seeds 1,2,3 --factions fac_france,fac_england`, 400 premiers tours, IA aux commandes)
France : impôts exceptionnels 4,6 / 2,2 / 2,9 % des recettes ; armée (entretien + sièges) 11,1 / 20,4 / 14,6 % des dépenses ;
sessions : sans objet (aucune assemblée). Angleterre : 2,2-2,5 % ; armée 13-14 %.
Détail France (moyenne par saison, graine 1) : revenu 36,7 k, armée 4,3 k, bâtiments 2,1 k, administration 7,4 k, maison 10,7 k,
obligations 7,0 k, intérêts 4,5 k.

## Mesure « après » (première version, avant réglage)
France graine 1 : exceptionnel 30 %, armée 3,3 % (l'IA ne dépense pas : la ponction d'épargne mange le surplus, administration 23,7 k),
55 sessions convoquées en 400 tours, 49 accordées, 6 refusées. Angleterre graine 1 : 31,6 %, 22 sessions toutes accordées.
Réglages suivants : palier « mesuré », IA qui vise un total en saisons (`ai.target_seasons`), `tax_efficiency` 0,07 -> 0,056.
