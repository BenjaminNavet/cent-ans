# bm-setcamp (options Settings de campagne)
Constat : à la base du lot, stance_fill, ai_moves_speed, autosave_interval, tutorial_*, auto_pause_on_alert (bataille), season_report, confirm_end_turn, news_filter, next_hint étaient déjà lus (lot setwire2). Seul `show_zoc` n'avait aucun consommateur.

| Option | Consommateur | Test |
|---|---|---|
| show_zoc | `campaign/armies/zoc.rs` (`ring_centres`, `draw_zoc`) : cercles rouges (gizmos drapés) sous chaque ennemi en vue quand une armée du joueur est sélectionnée ; survol d'un ennemi = son cercle, indépendant de l'option (comme Godot) ; plafond et couleur de `data/map/stance_cues.json` `zoc` ; rayon = `zoc_radius_km` × px/km | `show_zoc_option_gates_the_enemy_rings_not_the_hover_ring`, `tuning_reads_the_cues_file` |
| stance_fill | provinces/overlay.rs `stance_fill_enabled`, modes.rs | `stance_fill_option_reads_the_setting`, `stance_fill_off_keeps_the_heraldic_tint_only` |
| ai_moves_speed | hud/replay.rs `begin` (0 = pas de rejeu), flow.rs | `ai_moves_speed_zero_plays_no_replay` |
| autosave_interval | hud/flow.rs `autosave_name_for` | `autosave_slots_rotate_every_interval` |
| confirm_end_turn | hud/flow.rs `ConfirmMode` | `confirm_end_turn_modes` |
| season_report | hud/report.rs `ReportMode` | `season_report_mode_decides_modal_toast_or_nothing` |
| news_filter | report.rs `NewsScope`, chronicle `NewsMode` | `news_filter_scopes_the_world_rubric` |
| next_hint | hud/hint.rs | `next_hint_option_switches_the_card_off` |
| tutorial_* | progress/tutorial.rs (autostart, sync), menu/faction_select.rs `invite_wanted` | tests existants du tutoriel |
| auto_pause_on_alert | battle/hud/alerts.rs (lot setbattle) | `auto_pause_only_for_own_rout_or_fallen_general` |

Écart : cercles ZOC en polyligne (Gizmos) au lieu d'un Decal texturé (pas de Decal côté Bevy du dossier campaign) ; pas d'émission.
Reste : preuve par dump non faite (pas de section ZocRings dans le dump).
