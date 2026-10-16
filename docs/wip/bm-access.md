# bm-access (accessibilité, client Bevy)
Fait : symboles de moral ▲■▼ sur les cartes de régiment (battle/hud/cards.rs) en daltonien ; hachures du lavis de faction (provinces/shaders/fill.wesl, `OverlayParams.hatch`, `sync_hatch`) ; cadre contraste (ui/kit/contrast.rs : Outline + vélin éclairci) ; text_size appliqué en direct aux textes déjà affichés. Preuve : tests unitaires + dump Campaign (settings daltonien+contraste+1,3) : `ui_kit.access`, `fill.hatch=true`, `settings.palette`.
Écarts : bords des panneaux peints = Outline (pas d'épaississement de texture) ; hachures en espace écran, 3 directions par hachage de couleur ; symboles de moral non vérifiés en bataille (dump `morale_symbols` ajouté, tests seulement).
Restes : hachures des plaques ennemies (army_strip.gd), symboles de relation diplomatique en daltonien (map_mode_controller.gd `_place_relation_markers`, diplomatic_stances.gd), `show_zoc`.

## access2 (suite)
Fait : `AccessLabel` (ui/kit/contrast.rs) : symbole devant un texte en daltonien, en direct (⚔ ⚭ ⚜ relations, ▲■▼ moral) ; appliqué aux cartes de régiment du panneau d'armée (moral, = ▲■▼ de army_strip.gd) et au statut diplomatique (liste des factions + en-tête, comme diplomacy_faction_list.gd) ; `provinces/relation_markers.rs` : symboles ⚔/⚭/⚜ par province en mode Diplomatie (pool de textes écran, comme _place_relation_markers) ; dump `provinces.relation_markers`.
Écart : army_strip.gd n'a pas de hachures de plaques ennemies (seulement le symbole de moral) ; la mission les citait, rien à porter. Les plaques d'armée sur la carte n'ont pas de moral.
Restes : pastilles de légende du mode Diplomatie avec symbole (la légende Bevy est un modal BBCode, help.rs), `show_zoc`.
