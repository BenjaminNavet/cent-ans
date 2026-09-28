# FE4d — Registre féodal de l'Ibérie (1337)

Branche `feat/fe4d-iberie`, issue de `main` (fba2e7ce). Recette F4a (`docs/wip/fe4a-france.md`,
`docs/wip/fe4a-assets.md`). `CARGO_TARGET_DIR=core/target-fe4d`.

## Entités retenues (recherche web faite avant écriture, sources en anglais/espagnol/catalan)

8 nouvelles factions jouables, ~10 nouvelles provinces, plusieurs titres « en titre » sans
nouvelle faction (Valence, Catalogne/Barcelone, Sardaigne-Corse, Galice, León — tenus
directement par la couronne concernée, comme Valois/Charolais dans FE4a).

1. **fac_majorca** — Royaume de Majorque, indépendant jusqu'en 1343-44 (Jacques III).
   Provinces : `prov_mallorca` (existante, réassignée), `prov_roussillon` (existante,
   réassignée depuis fac_aragon), `prov_cerdagne` (NOUVELLE, scindée de Roussillon).
2. **fac_villena** — Seigneurie de Villena (rang approximé county, cf. Albret F4a), Juan
   Manuel, quasi-souverain. `prov_villena` (NOUVELLE, scindée de Murcie).
3. **fac_lara** — Seigneurie de Lara + Vizcaya (Biscaye), Juan Núñez III de Lara (réconcilié
   avec Alphonse XI en 1337, tient Lara et Vizcaya par son mariage avec María de Haro).
   `prov_lara` (NOUVELLE, scindée de Castille-Vieille) + `prov_biscay` (existante, réassignée
   depuis fac_castile).
4. **fac_urgell** — Comté d'Urgell, Jacques Ier d'Urgell (1327-1347), fils cadet d'Alphonse IV,
   oncle de Pierre IV. `prov_urgell` (NOUVELLE, scindée de Barcelone).
5. **fac_pallars** — Comté de Pallars, Arnau Roger II (1328-1343). `prov_pallars` (NOUVELLE,
   scindée d'Aragon).
6. **fac_ribagorca** — Comté de Ribagorce et d'Empúries, l'infant Pierre d'Aragon (1325-1342,
   oncle de Pierre IV). `prov_ribagorca` (NOUVELLE, scindée d'Aragon) + `prov_emporda`
   (NOUVELLE, scindée de Barcelone).
7. **fac_luna** — Seigneurie de Luna (rang approximé county), Lope de Luna (futur premier comte
   de Luna en 1348 ; en 1337 encore simple sire — anachronisme signalé). `prov_luna` (NOUVELLE,
   scindée d'Aragon).
8. **fac_calatrava** — Ordre militaire de Calatrava, pas de grand maître nommé (`ruler` omis,
   champ optionnel du schéma) faute de source fiable pour 1337 précisément. `prov_calatrava`
   (NOUVELLE, scindée de Tolède, autour de Calatrava la Nueva/Almagro/Ciudad Real).

Titre-seulement (pas de nouvelle faction, tenus directement par la couronne) :
- `tit_valencia` (royaume), provinces `prov_valencia` + `prov_alicante` (NOUVELLE, scindée de
  Valence — ancienne « Governació d'Oriola »).
- `tit_catalonia` (principauté→county, rang schéma), `prov_barcelona` (restante après scissions).
- `tit_sardinia_corsica` (royaume, titre seul, conquête en cours), `prov_sardegna` existante.
- `tit_galicia`, `prov_galicia` existante ; `tit_leon`, `prov_leon` existante — tenus par
  fac_castile.
- `tit_vizcaya` (Biscaye, 2e titre de fac_lara), `prov_biscay`.

Candidats écartés (source insuffisante pour un nom/date fiable en 1337) : vicomté de Cardona,
comté d'Empúries en ligne propre (déjà résolu ci-dessus, absorbé par la couronne dès 1325),
Albuquerque (seigneurie concédée après 1337), maison de La Cerda (prétention documentée mais
assise territoriale 1337 floue), ordre de Santiago (domaine trop dispersé pour une province).

## État
- [ ] Provinces/titres/factions/colonies écrits
- [ ] Héraldique, front_end, portraits
- [ ] Pipeline géo régénéré
- [ ] Tests Python/Rust verts
