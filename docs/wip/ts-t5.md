# TS T5 — armées (style Total War)

Branche `ts/t5`. Plan : `docs/wip/ts-style-total-war.md`.

## État
- Fait (compile, clippy ok) : path.rs (tirets animés, reach_split, blocked), zoc.rs (Ring, pulsation), standard.rs (banner_scale 1.5), données + schémas, dump. Tests lib 1337 ok, audio ok sans MUTE, smoke ok (crecy flaky une fois, ok au second passage), dump ok (banner_scale 1.5, 13 étendards, zoc_radius_px 11.1). Reste : jugement visuel par la session principale.

## Plan
1. `path.rs` : `reach_split` / `blocked` dans `PathView`, découpage en tirets animés (`dash_pieces`), `PathStyle` (données `stance_cues.json` bloc `path`). Dessin dans `orders::draw_overlays`.
2. `zoc.rs` : `Ring {centre, hostile}`, anneau pulsé, alliés/propres discrets au survol ou sélection. Rayon = `zoc_radius_km` du cœur.
3. `standard.rs` : `standard_banner.scale` (campaign_army_walk.json) agrandit hampe + drapeau.
4. Dump : `path_segments_*`, `zoc_rings_*`, `banner_scale`.

## Prochaine étape
Jugement visuel (sélection d'une armée + survol).
