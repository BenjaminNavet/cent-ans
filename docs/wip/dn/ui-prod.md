# DN ui-prod (branche dn/ui-prod)

État : A fait (catalogue icons_ink.json : 9 religions, 4 ordres, 6 édits, 13 glyphes/pictos, section `cursors_campaign` ; schéma et `ink_icons.build_cursors` généralisés).
Ids réels : religions rel_catholic, rel_catholic_rome, rel_orthodox, rel_islam, rel_judaism, rel_lollard, rel_hussite, rel_pagan, rel_armenian ; édits edict_market_freedoms et edict_militia_levy (pas market_franchise/levy du catalogue DN) ; ordres conformes.
Prochaine étape : B (génération sous `tools/gpu_lock.sh`), puis C (câblage).

## 10-08 (suite)
- C fait (code) : `InkGlyph` (ui/ink_glyph.gd, repli sur le glyphe si l'icône manque) branché dans map_ui (journal), season_report (en-têtes + KIND_STYLES.icon), tutorial, codex_hub, ransom_panel ; `CampaignCursor` (map/campaign_cursor.gd) lu par AttackCursor ; icônes `rel_*` dans l'encyclopédie, `ord_*` sur les boutons de chivalry_section, `edict_*` via ProvinceChoiceSection (get_icon automatique).
- Piège génération locale : la référence img2img au défaut (force 0,4) recopie la couronne de référence ; il faut `CENT_ANS_LOCAL_STRENGTH=0.15` (style de trait gardé, sujet respecté). `CENT_ANS_LOCAL_SEED_SALT=x` refait une image ratée.
- Génération : `scratchpad/pass_all.sh` (ink-icons par groupe puis event-art) sous `tools/gpu_lock.sh` ; verrou souvent tenu par dn_batch (attente longue).
- Inner class `GlyphIcon` existe dans general_seal.gd : d'où le nom `InkGlyph`.
