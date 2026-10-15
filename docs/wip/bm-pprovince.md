# bm-pprovince : panneau de province (Bevy)

État : lot province2 en cours de preuve. Panneau docké (Ville, Bâtiments, Colonies, Garnison) + `game-api::pprovince`.
Ajouts province2 : édits, La Table (régimes + Carême), vocation de colonie, texte de possession (couleur de position),
formation d'armée depuis la garnison (cases à cocher, plafond `army_rules.max_units`), fil d'Ariane cliquable
(`OpenPanel::Faction`), `StackedPanel` (Docked, id `province`) sur la racine ; monnaie via `fr::amount`.
Preuve : `CENT_ANS_OPEN_PROVINCE=capital CENT_ANS_FEATURES=dev CENT_ANS_PROFILE=debug client/run_bg.sh --state Campaign --script client/src/campaign/panels/province/proof_script.txt`
(section de dump `campaign_panel_province`).
Restes : icônes des options (IconLibrary), déroulé en menu flottant plutôt qu'en liste.
