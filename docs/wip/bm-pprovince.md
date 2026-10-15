# bm-pprovince : panneau de province (Bevy)

État : lot province2 en cours de preuve. Panneau docké (Ville, Bâtiments, Colonies, Garnison) + `game-api::pprovince`.
Ajouts province2 : édits, La Table (régimes + Carême), vocation de colonie, texte de possession (couleur de position),
formation d'armée depuis la garnison (cases à cocher, plafond `army_rules.max_units`), fil d'Ariane cliquable
(`OpenPanel::Faction`), `StackedPanel` (Docked, id `province`) sur la racine ; monnaie via `fr::amount`.
Preuve : `CENT_ANS_OPEN_PROVINCE=capital CENT_ANS_FEATURES=dev CENT_ANS_PROFILE=debug client/run_bg.sh --state Campaign --script client/src/campaign/panels/province/proof_script.txt`
(section de dump `campaign_panel_province`).
Ajouts province3 : barre d'emplacements par colonie (`game_api::pprovince::slot_bar`, onglets Colonies et Bâtiments,
cases icône + niveau, infobulle, clic = onglet Bâtiments de la colonie, `prov:slot:<colonie>:<racine>`), icônes des options
(La Table, édits, vocations), texte du Carême complet (valeurs `rule_constants` + lien Codex `cdx_careme`), descriptions
Table/édit avec mots du Codex cliquables (`codex::OpenCodex` via les `RichLink`).
Preuve : `proof_script3.txt` (sections `slot_bars`, `lent_text`, `linked_descriptions` du dump).
Touches : le panneau n'en a pas en propre (Échap = pile de panneaux) ; aucune action KeyMap ajoutée.
Restes : déroulé en menu flottant plutôt qu'en liste ; clic sur case vide = ouvre l'onglet, ne lance pas de chantier.
