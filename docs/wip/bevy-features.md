# Exploiter Bevy 0.20 (branche `perf/bevy-par-iter`, worktree `../gp-bevy-perf`)

Statut au 22/10 (fin de session) : lots 1-6 codés et fusionnés dans `main` (9985da963), non poussés. Solari désactivé par défaut. Mandat : tout activer par défaut, sauf ce qui casse (voir plus bas).

## Ce qui est dans main
- Features Cargo par défaut : `decals`, `solari` (`default = ["decals","solari"]`). `experimental_pbr_pcss` activée. Les 4 features `pbr_*_textures` sont RETIRÉES (limite Metal de 16 samplers).
- Réglages `data/fx/` : `battle_decals.json` (enabled), `solari.json` et `solari_campaign.json` (enabled:false), `postfx.json` (7 effets, tous `auto` = désactivés par préréglage), vent de bataille x3,5 (`battle_decor_look.json`).
- Dev : `bevy-devtools`, `hotpatch`, `reflect-inspect` (feature `dev` seulement).
- Campagne : fondus de `life` et statistiques déplacées dans `DumpPrepare` (lots L1/L2 de l'audit).
- Sol de bataille : gardes de distance, copies CPU libérées, sampler propre réduit à 1 (reste albedo), lecture `textureLoad`/`bilinear_clamped` ; sol éclairable en différé (Solari).
- Instrumentation : `texture_census.rs` (sections de dump `textures`, `texture_loads`).

## Bloquants connus
1. CAMPAGNE : plante au 1er rendu sur Metal, 17 samplers > 16 (terrain.wesl : 10 samplers propres + 8 de la vue forward). Lot `camp-samplers` en cours.
2. SOLARI en bataille : exit 0 mais monde 3D NOIR (UI seule). Lot `solari-noir` en cours. Défaut `enabled:false` tant que non réglé.
3. Prototype Solari campagne (`campaign/terrain/solari_proxy.rs`, terrain de substitution 500 k triangles) : non vu à l'écran, non mesuré.

## Notes d'étude (dans les worktrees `.claude/worktrees/agent-*` et branches `worktree-agent-*`)
- `bevy-audit-maison.md`, `bevy-etude-sol.md` (S3 cuisson KTX2 en tableau : à faire, textures = ma charge), `bevy-etude-foule.md` (lot 0 : mesurer la part des soldats), `bevy-etude-vent.md` (V0 mesurer), `bevy-etude-solari-campagne.md`, `bevy-audit-campagne.md` (L3-L7 restants), `bevy-meshlet.md` (infaisable végétation).

## Prochaine étape
Suivre les deux lots en cours, puis : (a) benchs sur machine calme (réf, Solari bataille, Solari campagne d=20/60/150) ; (b) captures de contrôle ; (c) étude terrain en ExtendedMaterial (à relancer, la création du worktree avait échoué, disque) ; (d) cuisson KTX2 en tableau ; (e) lots L3-L7 campagne.

## Règles apprises
- Worktrees : lier `data/map/pyramid` -> `~/dev/cent-ans-raw/compose/pyramid` et `rsync --ignore-existing` de `game/assets` depuis le dépôt principal, sinon captures et lancements faux.
- Disque : `CARGO_INCREMENTAL=0`, supprimer `client/target` des worktrees d'agents terminés.
- Tuer les clients par pid (`pkill -f` ne marche pas).
- Un seul banc release à la fois ; Mac/Metal d'abord (ADR/mémoire).
