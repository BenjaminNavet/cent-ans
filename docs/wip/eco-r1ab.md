# ECO-R1ab — « L'impôt consenti » (boucle juge 2)

Spec : `docs/audits/2026-10-29-economie-politique.md` § 6 R1 et § 8 (R1-a, R1-b). R1-c (IA, Angleterre, maltôte) et R1-d
(interface) viennent après : l'API publique de `sim-campaign::assembly` est prévue pour eux.

## Plan
- R1-a : `data/rules/assemblies.json` + schéma, `data-model::AssemblyRules`, `CampaignState::assemblies`,
  ordres `ConveneAssembly` / `AnswerAssembly`, `AssemblyError`.
- R1-b : `sim-campaign/src/assembly.rs` : vote, subside perçu par saison, concessions, promesses, humeur,
  impôts permanents, causes (`assembly_refused`, `promise_broken`, `subsidy_granted`), IA minimale.
  `LevyExtraordinary` refusé pour une maison qui a une assemblée (les autres gardent les impôts de `finance.json`).
- Rééquilibrage de l'ordinaire (`economy.json`) mesuré à la sonde séculaire (`eco_series`).

## État
- [x] Données, schéma, `data-model` (squelette).
- [ ] État + ordres + erreurs.
- [ ] Mécanique (vote, perception, concessions, promesses).
- [ ] Tests, causes, IA minimale.
- [ ] Mesure avant/après.

## Mesure « avant » (sonde séculaire, `eco_series --seeds 1,2,3 --factions fac_france,fac_england`)
À remplir.
