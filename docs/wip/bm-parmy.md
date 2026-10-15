# bm-parmy : panneau d'armée et recrutement (Bevy)

État : terminé et prouvé par scripts (voir rapport). Dump : section `panel_army`.
Crochets dev : `CENT_ANS_OPEN_RECRUIT=<province|colonie>`, `CENT_ANS_OPEN_ARMY=<id|player>`.
Fichiers : `client/src/campaign/panels/army/{mod,view,basket}.rs`, `core/crates/game-api/src/recruit.rs`.

Restes : fiche détaillée d'un régiment (clic droit, `unit_details_requested`), menu des armées étrangères
(bandeau en lecture seule), cartes sans image peinte (repli glyphe de catégorie), infobulles `[url]` Alt,
tri/onglets mémorisés entre colonies, vérification visuelle (aucune capture prise).
