# Lot HIST : corrections historiques (état)

Faites : capitales de 11 provinces (Touraine, Tlemcen, Grenade, Svealand, Vestlandet, Finlande, Yorkshire, Brest, Vitebsk, Syrmie ; kinds city/town permutés dans les colonies) ; peste noire (Raguse, Messine) ; Berwick anglaise ; Castille-Mérinides en trêve ; blasons Lancastre/Sienne ; note alliance anglo-impériale 1337-1341 ; id Rodolphe IV de Bade ; 7 couples 1337 + test `hist_spouses` ; apostrophes du tutoriel ; élisions Rust/client.

Refusées : Autriche (Vienne sans colonie), Nîmois (Nîmes sans colonie), Murcie (idem), Attique, Gwynedd (incertain), Armagnac (hors liste).

Restant : 2 tests pytest en échec à examiner (`test_heraldry_houses::test_every_character_house_has_arms` : maisons des nouveaux personnages sans blason, ex. Dunbar ; `test_portrait_archetypes::test_references_are_consistent`) ; clippy et tests workspace complets au merge ; images d'héraldique Lancastre/Sienne à régénérer.
