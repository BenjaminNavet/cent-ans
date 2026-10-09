# WH armya — état

Branche wh/armya. FAIT : entretien croissant + chef requis (ADR 0272), rayon de renfort + prévision « lève le siège » (0273), allure par composition. Tests : core/crates/sim-campaign/tests/armies/wh_armya.rs, src/tests/wh_armya_tests.rs. Sonde campaign_probe avant/après dans l'ADR 0272.
Reste / ouvert : renforts lointains à pleine puissance et sans dépense de leur mouvement (pas de vagues en 3D) ; IA ne nomme un chef qu'à la création par surplus de garnison ; vérification visuelle de l'info-bulle et de la ligne « à N km » (non faite, pas de capture).

IA : un général libre est cherché dans la province de la place avant CreateArmy (les personnages ne se déplacent pas seuls : point ouvert pour le lot chars), au plus `free_armies` armées sans chef sinon.
