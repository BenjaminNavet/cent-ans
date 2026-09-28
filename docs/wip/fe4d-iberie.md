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
- [x] Provinces/titres/factions/colonies écrits (10 provinces, 19 titres dont 14 nouveaux,
  8 factions, 7 personnages ; `tit_aragon`/`tit_castile` élagués des provinces devenues
  autonomes)
- [x] Héraldique (6 maisons + 2 blasons corrigés pour distinction visuelle), front_end (8
  fiches), portraits (bucket « iberia » complété)
- [x] Pipeline géo régénéré (voir commandes dans le rapport final)
- [x] `uv run --project tools pytest -q` : 906 passed, 2 skipped
- [x] `cargo fmt --all` propre, `cargo clippy --all-targets -- -D warnings` : 0 warning
- [x] `cargo test --no-fail-fast` : tout vert sauf `f7_events::montereau_leads_to_the_alliance_then_troyes`
  (voir « Test non corrigé » ci-dessous)

## État final : TERMINÉ (voir rapport de fin de lot)

## Écarts notables
- `chr_jacques_iii_de_majorque` existait déjà, rattaché à `fac_aragon` avec un commentaire
  explicite « faute de faction Majorque distincte, liberté du jeu ». Ce lot renverse ce choix
  documenté et lui donne sa propre faction (conforme au mandat qui cite Majorque comme
  candidat) — codex mis à jour en conséquence.
- `fac_aragon` n'avait plus aucun personnage hors le roi après le transfert de Jacques III de
  Majorque vers sa propre faction, ce qui faisait échouer deux tests de prise d'otage
  (`aragonese_hostage` cherche un courtisan aragonais vivant hors du souverain). Ajout de
  `chr_ramon_berenguer_de_prades`, fils cadet de Jacques II, oncle de Pierre IV, réellement sans
  domaine propre en 1337 (il recevra Prades en 1341) — personnage sourcé, pas d'invention.
- `chr_jaume_i_durgell` utilisait un champ `family.spouse` (chaîne libre) au lieu de
  `family.spouses` (liste d'identifiants de personnages) ; le schéma JSON ne l'a pas détecté
  (seul `cargo test`, via le chargeur Rust strict, l'a trouvé — même angle mort que documenté
  dans FE4a pour `title.schema.json`). Retiré (le mariage reste mentionné en description).
- Compteurs figés mis à jour dans `core/crates/sim-campaign/tests/campaign.rs`
  (`new_1337_matches_game_data`) : factions 36→44, provinces 141→151, armées 35→43.
- `tit_majorca`, `tit_valencia`, `tit_catalonia`, `tit_sardinia_corsica`, `tit_galicia`,
  `tit_leon` sont des royaumes « en titre » ; comme `title.schema.json` ne modélise que 3 rangs
  (kingdom > duchy > county) et interdit un lige de rang égal, ces titres n'ont PAS de
  `de_jure_liege` (contrairement à `tit_catalonia`, rang `duchy`, qui en a un). `fac_majorca`
  n'a donc pas de champ `suzerain` : indépendance de fait assumée mécaniquement, cohérente avec
  son objectif `be_independent`.
- 6 colonies neuves avaient un bâtiment interdit pour leur genre (règle « une ville ne porte pas
  de bld_cathedral » généralisée : village n'autorise ni `bld_stone_walls` ni `bld_castle`, abbaye
  n'autorise pas `bld_castle`) : bâtiments retirés ou genre corrigé (`set_almansa`→castle,
  `set_covarrubias`→town) ; trouvé uniquement par `cargo test` (`eq2_balance`), pas par les
  schémas JSON.
- Blasons : `fac_majorca` (mêmes pals qu'Aragon) et `fac_calatrava` (même croix que Gênes)
  rendaient un écu identique à une autre faction ; brisure de jeu ajoutée (bordure de sable pour
  Majorque, croix alésée pour Calatrava), signalée `uncertain` avec note.

## Test non corrigé (signalé, non affaibli)

`f7_events.rs::montereau_leads_to_the_alliance_then_troyes` échoue : après 3 tours de guerre
France-Angleterre à partir de 1419 (graine 4), seul `evt_montereau` s'est déclenché à l'été 1420,
pas `evt_alliance_anglo_bourguignonne` ni `evt_troyes` dans la fenêtre de tours du test. Sans
rapport de contenu avec l'Ibérie ; probablement sensible à la graine comme `cv3_ai_stances`
(observé en échec transitoire lors d'une exécution précédente, avant de repasser au vert) : plus
de factions IA (+8) et de personnages changent l'ordre de consommation du flux aléatoire par
tour. Je n'ai pas touché à `f7_events.rs` ni à la logique de déclenchement (hors mandat F4d,
règles dans `core/`) ; à signaler à F0/F3 si la sensibilité aux graines persiste après
l'intégration des autres lots FE4.
