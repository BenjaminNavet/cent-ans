07:24 bspeech 0ef07d5a8 clippy ok, 792 tests ok, smoke battle+crecy ok (battle 236s, charge machine) | base smoke: 9/10 ok, campaign_turns flaky (fail 15s, rerun ok)
07:35 bhud4 b50ca7ab2 + kbfree 83348359e clippy ok, 796 tests ok, smoke battle/campaign/campaign_turns ok
07:52 folk2 5846e787b OK clippy(all-targets)+tests 796+11 + smoke campaign campaign_descent, no fixes
07:56 settings4 b9bd89621 OK clippy/tests/smoke(main_menu), no fixes
08:01 uiinfra 6702624a9 OK clippy/tests(809)/smoke main_menu faction_select codex campaign, no fixes
08:07 econ ea3b04277 OK clippy(all-targets)/tests(809+)/smoke campaign campaign_turns, no fixes
08:14 mapui 5d601979c OK clippy(all-targets)+tests 810+10 + smoke campaign campaign_turns, no fixes
08:40 setwire ef892ba58 OK clippy/tests(812)/smoke main_menu battle crecy campaign campaign_descent (batch), fix: wip note conflict bm-settings.md (both sections kept)
08:40 bhud5 41fa24530 OK clippy/tests(819)/smoke battle crecy (batch), no fixes
08:40 veg4 93ae3a28b OK clippy/tests(844)/smoke campaign campaign_descent (batch), no fixes
08:40 townfar 233cbc731 OK clippy/tests(853)/smoke campaign campaign_descent (batch), no fixes
08:54 integ2 42045ef44 OK clippy/tests(854)/smoke main_menu faction_select campaign campaign_battle battle codex (batch), no fixes
08:54 perf d2f30e52f OK clippy(+profile feature)/tests(855)/smoke campaign campaign_descent battle (batch), no fixes
09:13 menu4 81f9037e0 OK clippy/tests(855)/smoke main_menu codex (batch), no fixes
09:13 diplo4 45be91369 OK core fmt/clippy/test game-api + client clippy/tests(857)/smoke campaign (batch), no fixes
09:13 bkeys b0f55668d OK clippy/tests(861)/smoke battle crecy (batch); settings4 ids (battle_group_n, battle_deselect, marks, map_pan/rotate) all resolve, no fix
09:13 bdecor5 115503166 OK clippy/tests(862)/smoke siege (batch), no fixes
09:35 refshots c5fc68725 OK clippy/tests(862)/smoke campaign campaign_descent (batch), no fixes
09:35 diplo5 6591e579c OK clippy/tests(864)/smoke campaign campaign_turns (batch); conflict diplomacy/mod.rs plugin build (kept HEAD read_fields/restore_focus/keyboard_free + NotifyPlugin + pick_lord)
09:35 setwire2 2d37210cf OK clippy/tests(873)/smoke campaign campaign_turns battle main_menu (batch); wip note conflict kept both sections; confirm_end_turn did not block campaign_turns, no fix
09:45 mats 9fa3bca5f OK clippy/tests(877)/smoke campaign campaign_descent battle siege (muted); cherry-pick 1028e3a50 auto-deduplicated, no fixes
09:50 models 5b61c721c OK clippy/tests (smoke after batch), no fixes
09:56 uibattle 753934d5e OK clippy/tests/smoke(campaign campaign_descent battle crecy siege) + alerts.rs conflict resolved (kept auto-pause + new slot parent)
10:04 campvis ce664d815 OK clippy/tests(898)/smoke campaign campaign_turns (muted); conflicts camera/mod.rs (kept campvis frame_player_capital + HEAD Option<RenderQuality>) and bm-fidelity.md (both sections), no fixes
10:32 vegvis a29a8a004 OK clippy/tests(890)/smoke (batch, muted); conflicts camera/mod.rs (kept HEAD frame_player_capital, cherry-pick 1028e3a50 dedup) and vegetation/render.rs (vegvis MODEL_LOD + model_params); glb tree models back on by default (vegvis foliage treatment, CENT_ANS_VEG_MODELS=0 to disable)
10:32 uicamp 61f651d53 OK clippy/tests(896)/smoke (batch, muted); camera/mod.rs cherry-pick dup conflict kept HEAD, no fixes
10:32 siegemat 2a0182bc0 OK clippy/tests(904)/smoke campaign campaign_descent campaign_turns siege battle (muted), no conflicts, no fixes
10:39 perf2 c4698813f OK clippy/tests(904)/smoke after batch, no conflicts, no fixes
10:47 parch da088f76b OK clippy/tests(914)/smoke after batch; Cargo.lock conflict (kept tracing+tracing-subscriber and tiny-skia), no fixes
10:53 bvis 9c62b1a8a OK clippy/tests(914)/smoke after batch; conflicts works.rs (kept HEAD siegemat atlas materials over bvis wall textures) and bm-fidelity.md (both sections), no fixes
10:58 launcher 9d90943bb OK pytest test_launch_update (11) + batch smoke (muted) campaign campaign_descent main_menu battle crecy siege campaign_battle all 7 OK, no conflicts, no fixes
11:02 nofocus 4ddf8d7b6 OK clippy/tests(925)/smoke main_menu campaign battle (muted, new offscreen default via run_bg.sh), no conflicts, no fixes
11:16 weather4 c1cefcee4 OK clippy/tests(916)/smoke after batch (muted), no conflicts, no fixes
11:16 water3 5c8a9630d OK clippy/tests(916)/smoke campaign campaign_descent battle siege (muted), no conflicts, no fixes
11:24 campvis3 a1962d14c OK clippy/tests(930), smoke after batch; conflict bm-fidelity.md (both sections, dup title dropped); carries campvis2, no fixes
11:44 dataassets 498fb3f24 OK clippy/tests(934)/pytest schemas(1214)/smoke main_menu campaign campaign_descent battle (muted); Cargo.toml conflict (kept tiny-skia + 2 deps); fix 2745ed3d0 (CampaignLightingDoc feeds LightData/FogLook/ShadowTuning on reload)
12:08 devtools1 cd51bd828 OK clippy/tests(923+snapshots 3, sccache, fresh target)/smoke main_menu campaign battle (muted); INDEX.md conflict; ADR renumbered 0362->0365 (coordinator), INDEX lists 0362/0363/0365
12:15 borders2 716749f56 OK clippy/tests(941), smoke after batch; conflicts camera/mod.rs (kept HEAD Option<RenderQuality> MSAA + campvis frame_player_capital) and bm-fidelity.md (HEAD kept, dup title dropped), no fixes
12:28 towns2 7f361486e OK clippy/tests(947)/smoke campaign campaign_descent campaign_turns (muted, batch with borders2), no conflicts, no fixes
12:42 campvis2 a1962d14c OK already carried by campvis3 (no-op merge), worktree/branch removed
12:42 bvis2 4f9c64b19 OK clippy/tests(939)/pytest schemas(1215)/smoke campaign campaign_descent battle crecy siege (muted); skinned.rs conflict (kept HEAD prepare_buffers run_if + bvis2 queue_shadow_casters), no fixes
12:55 hudfix 26699b830 OK clippy/tests(954)/smoke campaign campaign_descent campaign_turns main_menu (muted, batch with aflags), no conflicts, no fixes; note: audio_playback test fails if CENT_ANS_MUTE=1 is exported during cargo test
12:55 aflags c28eb5d74 OK clippy/tests(961, full run)/smoke after batch (muted), README auto-merged, no fixes
13:06 uiimg 44528a41f OK clippy/tests(968)/smoke campaign campaign_turns battle (muted, batch with uisound), no conflicts, no fixes
13:06 uisound c5dd606d7 OK clippy/tests(974, full --features dev)/smoke campaign campaign_turns battle (muted); INDEX.md conflict (kept HEAD 0365 tools numbering), no fixes
13:26 fxfire 91bde7e5c OK clippy/tests(982)/smoke after batch (muted), README auto-merged, no fixes
13:26 uileft 01afbc47a OK clippy/tests(985)/smoke campaign battle siege main_menu (muted; main_menu flaky 2s exit once, retry ok); fix 10a4ef7ad advisor show_bubble Option<Res<UiTheme>> (audio_playback panicked: missing resource)
13:41 ga3 9f468a576 OK clippy/tests(997)/smoke after batch (muted); conflict siege/mod.rs dump (kept fire_emitters_started + ga3 fields), no fixes
13:41 svgconv 1fded7101 OK clippy/tests(1001)/pytest svg_png(2)/smoke siege battle crecy main_menu faction_select campaign (muted); conflict chronicle/mod.rs (seal+rubric row then decision art), no fixes
13:48 bvis3 bfb89d069 OK clippy/tests(1003)/smoke battle crecy siege main_menu (muted); conflicts decor/mod.rs (kept props+panorama) and bm-fidelity.md (bvis3 headings + HEAD siegemat note), no fixes
14:02 fg3 a2a265753 OK clippy/tests(1007, full --features dev)/smoke after batch (muted), clean merge, brought bm/bherald (not yet merged separately), no fixes
14:02 council c1cdf86ff OK clippy/tests(1012)/smoke battle crecy siege campaign (muted); fix 8bac557a9 dump_council try_query unwrap panicked in all 4 smoke scenarios
14:17 lifeanim f642e4240 OK clippy/tests(1016)/pytest schemas(1690)/smoke after batch (muted); INDEX.md conflict (kept HEAD 0362-0365), no fixes
14:17 campwater a3a99e4c0 OK clippy/tests(1024, full --features dev)/smoke campaign campaign_descent campaign_turns (muted), no conflicts, no fixes
14:18 bherald a2a265753 OK already carried by fg3 merge (ancestor), worktree/branch removed
14:50 devtools2 05d6e9947 OK clippy+tests with and without --features dev (1024, snapshots 3)/smoke battle crecy siege main_menu campaign (muted); conflicts Cargo.toml (dev = file_watcher + devtools2 deps), main.rs (DataSourcePlugin + tune_default_plugins), INDEX (0362-0365), Cargo.lock regenerated; ADR 0364 ref 0362->0365; fix 83ed754ef insta sort_maps (dev turns on serde_json preserve_order)
15:00 campvis4 ac91341f1 OK clippy/tests(1029, full --features dev)/smoke campaign campaign_descent campaign_turns (muted), clean merge after campwater (terrain bindings 0-26 unique), no fixes
11:58 setbattle d04fd675e OK clippy/tests(1033) in bm/merge8, smoke after batch, no conflicts, no fixes
12:00 diplo6 merge 3ec8994f0 RESET (pause requested by player before checks finished); bm/merge8 back at d04fd675e, worktree/branch kept
14:21 diplo6 4890ab7cf OK clippy/tests(1054, batch diplo6+fxsmoke+naval2+bhud6, merge8) clean merges, no fixes; smoke at end
14:21 fxsmoke 76742e890 OK clippy/tests(1054, batch diplo6+fxsmoke+naval2+bhud6, merge8) clean merges, no fixes; smoke at end
14:21 naval2 4e51870c4 OK clippy/tests(1054, batch diplo6+fxsmoke+naval2+bhud6, merge8) clean merges, no fixes; smoke at end
14:21 bhud6 a04c78916 OK clippy/tests(1054, batch diplo6+fxsmoke+naval2+bhud6, merge8) clean merges, no fixes; smoke at end
14:26 uisound2 9fef974b5 OK clippy/tests(1067, batch uisound2+setcamp+flowinteg+uileft2, merge8); conflict diplomacy/mod.rs (trade shortcut + panel_sound kept), fix 09e5bb700 too_many_arguments; smoke at end
14:26 setcamp 05b1b227d OK clippy/tests(1067, batch uisound2+setcamp+flowinteg+uileft2, merge8); no fixes; smoke at end
14:26 flowinteg f73d6d663 OK clippy/tests(1067, batch uisound2+setcamp+flowinteg+uileft2, merge8); no fixes; smoke at end
14:26 uileft2 fc1f8b734 OK clippy/tests(1067, batch uisound2+setcamp+flowinteg+uileft2, merge8); no fixes; smoke at end
15:01 uiimg2 d653e16ef OK clippy/tests(1082 full --features dev, batch uiimg2+access+econ2+council2, merge8); conflict hud/journal.rs vs uileft2 (vignette + genre bar, both dump fields, both tests); core clippy/tests sim-campaign+game-api (1145) OK
15:01 access ef4b1a990 OK clippy/tests(1082 full --features dev, batch uiimg2+access+econ2+council2, merge8); no conflicts, no fixes
15:01 econ2 def8c1047 OK clippy/tests(1082 full --features dev, batch uiimg2+access+econ2+council2, merge8); no conflicts, no fixes
15:01 council2 2ad5df5b3 OK clippy/tests(1082 full --features dev, batch uiimg2+access+econ2+council2, merge8); no conflicts, no fixes
15:01 merge8 smoke campaign campaign_turns battle crecy siege main_menu OK (muted; siege timed out once under load ~95 with no app output, retry ok 31s)
15:11 bdecor5 3195cdb85 OK clippy/tests(1092 full --features dev, batch bdecor5+parch2+portaudit, merge8)/smoke battle crecy campaign (muted); no conflicts, no fixes
15:11 parch2 c7aa30628 OK clippy/tests(1092 full --features dev, batch bdecor5+parch2+portaudit, merge8)/smoke battle crecy campaign (muted); no conflicts, no fixes
15:11 portaudit d9d05f64a OK clippy/tests(1092 full --features dev, batch bdecor5+parch2+portaudit, merge8)/smoke battle crecy campaign (muted); no conflicts, no fixes
- access2 prêt (9c9324727)
- lifeanim2 prêt (3ab18d8fa)
- dataassets2 prêt (88235803a) — piège: adopter en OnEnter(Ready), pas LoadedWithDependencies
- FOURNÉE SUIVANTE: coastterr prêt (d5875e1bb). Reste: water/ pourrait réutiliser CoastTextures (coast_dist.png chargé 2x, 43 Mo GPU)
- FOURNÉE SUIVANTE: batmo prêt (559cc60aa) — touche data/fx/battle_lighting.json + schéma, tools/tests/test_schemas.py ; lancer pytest schémas
- FOURNÉE SUIVANTE: bwater prêt (b606f83af) — conflit probable scene/setup.rs avec batmo (deux hooks CENT_ANS_BATTLE_*), garder les deux
- FOURNÉE SUIVANTE: siegeland prêt (df91e175e) — touche units/skinned.wesl mode 3 (volley_time empaqueté) : vérifier avec perf3/bunits6/lifeanim2. Décision : battle_capture_stage.gd (outil de capture CLI) NON porté, remplacé par --script/dumps de run_bg.
- FOURNÉE SUIVANTE: armies6 prêt (76642b44a) — touche core game-api (reach.rs), camera/focus.rs, parchment/extras.rs
16:00 merge8 965a00664 OK into bevy-migration (wave 8 + bdecor5/parch2/portaudit), clean merge over showfix; clippy/tests(1108 after towns4+access2+lifeanim2+dataassets2)/core sim-campaign+game-api(1145)/smoke campaign campaign_turns campaign_descent battle crecy siege main_menu (muted)
16:00 towns4 af53ad8b8 OK clean merge; checks in the batch pass above
16:00 access2 5d5b404ee OK clean merge (carries access); smoke main_menu+campaign in the batch pass
16:00 lifeanim2 56aac8c3e OK conflict camp_horse.rs/.wesl vs bdecor5 (two ports of the same horses): kept lifeanim2's (adds shadow prepass, pose module), site.rs kept bdecor5 kit_plan.horse_pieces, dropped the duplicated camp_horses dump entry
16:00 dataassets2 c5365f619 OK clean merge
16:20 setwire3 fc1fcbd45 OK clean; clippy/tests(1113)/smoke main_menu faction_select campaign battle (muted)
16:20 aflags2 3304ff870 OK clean; same pass; agent/mod.rs fmt noise was only in the worktree (base already formatted, no fmt commit needed)
16:45 weather5 f88d4dfc2 OK clean; clippy/tests(1144)/smoke crecy siege battle campaign campaign_turns campaign_descent main_menu (muted)
16:45 bunits6 7a24d4f98 OK clean (no skinned conflict with lifeanim2); same pass
16:45 uiinfra3 2a7e3ec8f OK clean; c687ffb40 drops obsolete uiinfra line from bm-restes.md; same pass
16:45 audio4 8c970daeb OK clean; same pass; orphan fire_flipbooks diff saved to scratchpad/orphan_audio4_fire.patch
16:45 gore 7ec9a21cf OK conflict blood.rs vs setbattle: gore's data-driven `stains`, falling back to setbattle's pool_scale(level); same pass
17:10 mapmodes 15721d724 OK conflict provinces/mod.rs (relation_markers + shields); clippy/tests(1161)/game-api/smoke crecy siege battle campaign campaign_turns main_menu faction_select (muted)
17:10 campui7 0186e8e3b OK clean; fix 7bd7d46f8 migrate feudal_tutorial/done -> feudal_tutorial_done (+test)
17:10 border2 ad75b84f8 OK clean; same pass
17:10 perf3 fff33cca8 OK clean; stride = RECORD_BYTES everywhere (param block 131 vec4 is the uniform); same pass
17:30 finegeo 513ce853b OK clean; clippy/tests(1175)/core fmt+clippy+test full(1956)/smoke campaign campaign_turns campaign_descent main_menu (muted)
17:30 campui6 d12b446f3 OK clean; same pass
fire_flipbooks drift: bm_wt2.sh copied data/ from gp-bevy's tree before merge8 (pre-a4d95924d text); not real work, restored in each worktree before removal
17:50 menu5 40511a750 OK clean; clippy/tests(1182)/game-api clippy+tests/smoke main_menu faction_select campaign battle crecy siege campaign_turns (muted); fix: core fmt of start_sheets.rs (import order)
18:27 coastterr 0bfd0dd78 OK clean; clippy/tests(1225 --features dev)/core fmt+clippy+test full(1959)/pytest schemas(1216)/smoke campaign campaign_turns campaign_descent battle crecy siege main_menu (muted; main_menu timed out once under load with no app output, retry ok 4s)
18:27 batmo 7ce30563e OK clean; same pass (schemas ok)
18:27 bwater 9bdef1f57 OK clean (no setup.rs conflict: CENT_ANS_BATTLE_WEATHER + CENT_ANS_BATTLE_FIELD both kept); same pass
18:27 siegeland 62a2cfc70 OK clean; skinned.wesl mode 3 role fade uses existing prev_* vars and params.fx.x, RECORD_BYTES stride unchanged; same pass
18:27 armies6 7659f0ebe OK clean; core game-api in the full core pass; same pass
18:27 campwater2 41ec0a92b OK clean (carries finegeo + gore); same pass; no fire_flipbooks drift in any worktree
