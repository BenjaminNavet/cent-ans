# H10 — Codex : Table, cuisine, plantes, médecine

Branche : `worktree-agent-a038490f6abdd9c53`. Agent historien (alimentation, médecine).

## Périmètre
- 22 ids restants de `data/codex/_diet_links.md` (`cdx_viandier`, `cdx_gabelle` existent déjà).
- 22 ids de `data/codex/_herb_links.md`.
- `cdx_hypocras`, `cdx_medecine_medievale`, `cdx_epices_medievales` (retirés de `_todo.md`).
- Complément : Ménagier, Forme of Cury, verjus, cervoise, tranchoir, humeurs et cuisine,
  Chauliac, Montpellier, Salerne, apothicaires, barbiers, léproseries, hôtels-Dieu, uroscopie,
  saignée, Hildegarde, Compendium de epidemia, quarantaine de Raguse.

## État
- [x] lot 1 : pain_bis, four_banal, potage, assolement_triennal, legumineuses, careme, jours_maigres, hareng
- [x] lot 2 : hanse, bataille_des_harengs, salaison, beurre_de_careme, tour_de_beurre, vin_de_gascogne, clairet, commerce_de_bordeaux

- [x] lot 3 : taillevent, cameline, poivre, maniguette, blanc_manger, hypocras, epices_medievales, medecine_medievale (3 ids retirés de _todo.md)
- [x] lot 4 : plantes du jardin (sauge, rue, menthe, fenouil, ail, oignon, hysope, saule)
- [x] lot 5 : reine_des_pres, camomille, plantain, consoude, millepertuis, theriaque, aloes, safran
- [x] lot 6 : pavot, mandragore, jusquiame, genievre, vinaigre, romarin
- [x] lots 7-8 : 8 ids de _h10_links.md (menagier_de_paris, forme_of_cury, verjus, cervoise,
  tranchoir, uroscopie, saignee, hildegarde_de_bingen) ; les 12 autres ids du fichier étaient déjà
  écrits. `_h10_links.md` supprimé (ne restait aucun id).

## Décisions
- Alias « Taillevent » et « Guillaume Tirel » déplacés de `cdx_viandier` vers `cdx_taillevent`.

## Prochaine étape
Lot H10 terminé (toutes les fiches prévues sont écrites, validées par `validate_codex` et
`pytest tools/tests/test_codex.py`). Aucune suite prévue pour ce lot.
