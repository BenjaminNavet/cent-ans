# WIP — B5 Unités, navires, techniques (bulles partout)

Spec : `docs/design/2026-09-25-bulles-partout.md`. Branche : `worktree-agent-adf46adc6cef2e799`.

## Objectif
Une fiche Codex (`entity` = id) pour les 27 unités, 4 navires, 45 technologies ; `gameplay` chiffré
d'après `data/` et `core/crates` ; audit historique des JSON dans
`docs/histoire/audit-2026-09-25-unites.md`.

## État
- [x] Recherche des règles (capacités, effets de technologies, naval) dans `core/crates`
- [x] Fiches unités (27) : 20 nouvelles + 7 enrichies
- [x] Fiches navires (4)
- [x] Fiches technologies (45) : 29 nouvelles + 16 enrichies (tech_aqua_vitae passe de cdx_romarin à cdx_eau_de_vie)
- [x] Sources génériques des technologies remplacées, notes d'années corrigées
- [x] Audit écrit (`docs/histoire/audit-2026-09-25-unites.md`) + corrections JSON unités/navires + liens dans les descriptions
- [x] Validateur Codex vert (288 fiches), pytest vert (418)
- [ ] cargo test (en cours)
- [ ] Validateur Codex, pytest, cargo test

## Prochaine étape
Confirmer cargo test, puis fusion par l'orchestrateur. Coordination B4 : `cdx_jardin_des_simples`, `cdx_universite_paris`, `cdx_hotel_dieu` et `cdx_apothicaires` restent disponibles/déjà liés ; les technologies utilisent des fiches distinctes (`cdx_culture_des_simples`, `cdx_universites`, `cdx_hydraulique_medievale`, `cdx_moulin_a_pivot`, `cdx_fortification_artillerie`, `cdx_maconnerie_militaire`, `cdx_hygiene_urbaine`, `cdx_comptabilite_marchande`) sans alias de bâtiment.

Générateur : les textes « En jeu » sont produits par un script (chiffres lus dans `data/`), constantes de règles relevées dans `core/crates` (capacités `sim-battle/src/sim.rs`, économie `sim-campaign/src/economy.rs`, recherche `research.rs`, naval `sim-battle/src/naval/`). Champs sans effet en jeu (non mentionnés) : `shield_wall`, `wall_breach` (capacité), `mercenary`, `recruit_time_turns`, `cost.resources`, `tier`, `cost` des navires.

## Correspondance entités → fiches (76)

| Entité | Fiche |
|---|---|
| `ship_barge` | `cdx_barge` |
| `ship_cog` | `cdx_cogue` |
| `ship_galley` | `cdx_galee` |
| `ship_nef` | `cdx_nef` |
| `tech_aqua_vitae` | `cdx_eau_de_vie` |
| `tech_artillery_fortification` | `cdx_fortification_artillerie` |
| `tech_barber_surgeons` | `cdx_barbiers_chirurgiens` |
| `tech_bombards` | `cdx_premieres_bouches_a_feu` |
| `tech_bookkeeping` | `cdx_comptabilite_marchande` |
| `tech_brigandine` | `cdx_brigandine` |
| `tech_chauliac_surgery` | `cdx_guy_de_chauliac` |
| `tech_coat_of_plates` | `cdx_cotte_de_plates` |
| `tech_compagnies_d_ordonnance` | `cdx_ordonnance_louppy` |
| `tech_crossbow_windlass` | `cdx_arbalete_a_tour` |
| `tech_dismounted_tactics` | `cdx_tactique_anglaise` |
| `tech_double_entry` | `cdx_partie_double` |
| `tech_field_artillery` | `cdx_artillerie_de_campagne` |
| `tech_francs_archers` | `cdx_ordonnance_montils` |
| `tech_full_plate` | `cdx_harnois` |
| `tech_gothic_flamboyant` | `cdx_gothique_flamboyant` |
| `tech_gunpowder` | `cdx_poudre_noire` |
| `tech_hand_cannon_drill` | `cdx_wagenburg` |
| `tech_handgonnes` | `cdx_batons_a_feu` |
| `tech_hanseatic_trade` | `cdx_hanse` |
| `tech_herb_garden` | `cdx_culture_des_simples` |
| `tech_hospital_reform` | `cdx_hotel_dieu` |
| `tech_humoral_theory` | `cdx_humeurs` |
| `tech_leprosaria` | `cdx_leproserie` |
| `tech_letters_of_credit` | `cdx_lettre_de_change` |
| `tech_longbow_drill` | `cdx_entrainement_arc_long` |
| `tech_masonry` | `cdx_maconnerie_militaire` |
| `tech_montpellier` | `cdx_montpellier` |
| `tech_paper_mills` | `cdx_papier` |
| `tech_pavise` | `cdx_pavois` |
| `tech_plague_consilia` | `cdx_compendium_de_epidemia` |
| `tech_printing_press` | `cdx_imprimerie` |
| `tech_quarantine` | `cdx_quarantaine` |
| `tech_regimen_sanitatis` | `cdx_regimen_sanitatis` |
| `tech_royal_taxation` | `cdx_taille` |
| `tech_siege_engineering` | `cdx_ingenieurs_de_siege` |
| `tech_soporific_sponge` | `cdx_pavot` |
| `tech_standing_companies` | `cdx_armee_permanente` |
| `tech_theriac` | `cdx_theriaque` |
| `tech_three_field_rotation` | `cdx_assolement_triennal` |
| `tech_universities` | `cdx_universites` |
| `tech_urban_sanitation` | `cdx_hygiene_urbaine` |
| `tech_water_mills` | `cdx_hydraulique_medievale` |
| `tech_willow_bark` | `cdx_saule` |
| `tech_windmills` | `cdx_moulin_a_pivot` |
| `unit_bombard` | `cdx_bombarde` |
| `unit_breton_knights` | `cdx_chevaliers_bretons` |
| `unit_coutiliers` | `cdx_coutiliers` |
| `unit_crossbowmen` | `cdx_arbaletriers_des_villes` |
| `unit_culveriners` | `cdx_couleuvriniers` |
| `unit_ecorcheurs` | `cdx_ecorcheurs` |
| `unit_english_retinue` | `cdx_retenues_anglaises` |
| `unit_flemish_pikemen` | `cdx_piquiers_flamands` |
| `unit_francs_archers` | `cdx_franc_archer` |
| `unit_gascon_crossbowmen` | `cdx_arbaletriers_gascons` |
| `unit_genoese_crossbowmen` | `cdx_arbalete` |
| `unit_goedendag_militia` | `cdx_goedendag` |
| `unit_hobelars` | `cdx_hobelars` |
| `unit_jinetes` | `cdx_jinetes` |
| `unit_knights` | `cdx_chevalerie` |
| `unit_longbowmen` | `cdx_arc_long` |
| `unit_mangonel` | `cdx_mangonneau` |
| `unit_men_at_arms_foot` | `cdx_hommes_d_armes` |
| `unit_mounted_archers` | `cdx_archers_montes` |
| `unit_mounted_sergeants` | `cdx_sergents_montes` |
| `unit_ordonnance_gendarmes` | `cdx_compagnies_ordonnance` |
| `unit_routiers` | `cdx_routiers_des_compagnies` |
| `unit_scottish_spearmen` | `cdx_schiltron` |
| `unit_siege_tower` | `cdx_beffroi` |
| `unit_trebuchet` | `cdx_trebuchet` |
| `unit_urban_militia` | `cdx_milices_urbaines` |
| `unit_welsh_spearmen` | `cdx_lanciers_gallois` |
