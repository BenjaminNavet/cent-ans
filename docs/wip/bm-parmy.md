# bm-parmy : panneau d'armée et recrutement (Bevy)

État : terminé et prouvé par scripts (voir rapport). Dump : section `panel_army`.
Crochets dev : `CENT_ANS_OPEN_RECRUIT=<province|colonie>`, `CENT_ANS_OPEN_ARMY=<id|player>`.
Fichiers : `client/src/campaign/panels/army/{mod,view,basket}.rs`, `core/crates/game-api/src/recruit.rs`.

Restes : fiche détaillée d'un régiment (clic droit, `unit_details_requested`), menu des armées étrangères
(bandeau en lecture seule), cartes sans image peinte (repli glyphe de catégorie), infobulles `[url]` Alt,
tri/onglets mémorisés entre colonies, vérification visuelle (aucune capture prise).

## parmy2 (fiche de régiment, armée étrangère, infobulles Alt)

Fait : `army/detail.rs` (spec `TooltipSpec` d'un régiment, bulle courte au survol, détaillée épinglée au clic droit),
`recruit::regiment_detail` dans game-api (stats, forces/faiblesses, capacités), bandeau « Armée étrangère · lecture seule »
(aucun ordre émis, sélection d'une armée étrangère ouvre le panneau), `StackedPanel` (Docked) sur les deux racines,
règles `ib:rule:*` enregistrées au démarrage (Alt chaîne les liens). Dump `panel_army` : `read_only`, `bubbles`,
`chained_links`, `sheets_pinned`.
Restes : vérification visuelle, cartes sans image peinte, tri/onglets mémorisés.
