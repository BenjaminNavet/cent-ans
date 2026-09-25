# WIP — B4 Bâtiments et ressources (bulles partout)

Branche : `b4-batiments`. Conception : `docs/design/2026-09-25-bulles-partout.md`.

## Fait
- 30 bâtiments, 10 ressources : chacun a une fiche Codex avec `entity` et `gameplay` (chiffres tirés
  de `data/buildings/`, `data/resources/` et de `core/crates/sim-campaign/src` : buildings.rs,
  economy.rs, population.rs, religion.rs, research.rs, medicine.rs, orders.rs).
- Nouvelles fiches (33) : cdx_abbaye, cdx_collegiale, cdx_eglise_paroissiale, cdx_cathedrale,
  cdx_butte_de_tir, cdx_arsenal, cdx_boulevard_artillerie, cdx_chateau_fort, cdx_enceinte_de_pierre,
  cdx_palissade, cdx_marche, cdx_atelier_d_engins, cdx_foire, cdx_maison_des_metiers, cdx_draperie,
  cdx_comptoir_marchand, cdx_port, cdx_forge, cdx_haras, cdx_moulin_a_eau, cdx_moulin_a_vent,
  cdx_pressoir_banal, cdx_maison_de_fonte, cdx_adduction_d_eau, cdx_pierre, cdx_ble, cdx_vin,
  cdx_laine, cdx_drap, cdx_poisson, cdx_fer, cdx_bois, cdx_londres.
- Fiches enrichies (entity/gameplay/aliases/paragraphe) : cdx_apothicaires, cdx_jardin_des_simples,
  cdx_hotel_dieu (entity tech_hospital_reform -> bld_hotel_dieu), cdx_ost_feodal (champ de montre),
  cdx_enluminure (scriptorium), cdx_universite_paris (université générique), cdx_gabelle (sel),
  cdx_tin_stannaries (étain) ; cdx_places_fortes cède les alias « château fort ».
- Repères : seul Londres n'avait pas de fiche (cdx_londres, entity prov_middlesex).
- Descriptions des 30 bâtiments et 9 ressources corrigées et liées au Codex ; audit dans
  `docs/histoire/audit-2026-09-25-batiments.md`.
- Liens vers les fiches B2 (`cdx_jeu_*`) tolérés via `data/codex/_b4_links.md`.
- Validateur Codex vert, `uv run --project tools pytest` vert (418 tests).

## Reste / points ouverts
- `cargo test -p data-model -p sim-campaign` vert (crates qui lisent les données).
- À la fusion avec B2 : supprimer `_b4_links.md` si les 5 fiches `cdx_jeu_*` existent sous ces ids.
- Équilibrage à arbitrer (voir audit) : boulevard d'artillerie qui remplace le château fort ;
  `enables_units` purement indicatif ; piété/prestige des bâtiments non calculés ; coûts en pierre
  non prélevés ; `satisfies_classes: []` (fer, pierre) interprété par le code comme « toutes les classes ».
