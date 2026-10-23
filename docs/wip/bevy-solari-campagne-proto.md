# Solari en campagne : prototype (lot C de l'étude)

Étude : `docs/wip/bevy-etude-solari-campagne.md` (branche worktree-agent-a9ec5cac482fc79cc). Prototype de bataille : `client/src/battle/scene/solari.rs`.

## Principe (client/src/campaign/terrain/solari_proxy.rs, feature `solari`)

Réglage `data/fx/solari_campaign.json` (schéma `data/schemas/fx_solari_campaign.schema.json`) : `enabled`,
`terrain_proxy_triangles` (500 000), `albedo_from` (`composed_map` | `flat`), `flat_color`, `restir`, `max_bounces`,
`sky_intensity`. Sonde de capacité réutilisée : `battle::scene::solari::missing_features`.

- Proxy de terrain : grille régulière (~500 k triangles) échantillonnant `Surface::display_height_at` (jumeau CPU de
  `displayed_y`) + enfoncement des bords du vertex shader (pas de géomorphe ni de rivière creusée). `StandardMaterial`
  en rendu DIFFÉRÉ, `RaytracingMesh3d` sur le même Mesh (MAIN|RENDER). Reconstruit si l'exagération verticale
  (`ReliefState::vertical_scale`) bouge de plus de 5 % (au plus toutes les 1,5 s).
- Albédo : colormap BC1 relue sur disque dans un thread, un texel par bloc d'un mip grossier (~885 x 763), moyenne des deux
  couleurs de bloc. Sans grade saisonnier, neige, splat, brume.
- Nœuds du quadtree : `Visibility::Hidden` réappliqué après `update_lod` (logique intacte).
- Caméra : `SolariLighting` (requires Hdr, DeferredPrepass, DepthPrepass, MotionVectorPrepass), STORAGE_BINDING, `Msaa::Off`
  (forcé dans `RenderQuality.msaa_campaign`, donc caméra UI alignée), `EnvironmentMapLight` en dégradé de ciel (Solari ignore
  `AmbientLight`).
- Soleil : ombres cascades coupées (remplacées par les rayons).
- Matériau par défaut : forward partout (Solari impose le différé) ; seul le proxy est différé. Le prototype de bataille
  (s'il est actif) rétablit le différé à la sortie de la campagne. Les deux plugins protègent l'ajout de `SolariPlugins`.
- Post-traitement maison (`post.rs`) : gère déjà profondeur multisample ou non ; MSAA coupé donc variante simple.
- Dump : section `solari_campaign` (enabled, reason, proxy_triangles, build_ms, rebuilds, camera_deferred, albedo_texels).

## État / vérifications

- Tests unitaires (4) : grille, orientation/normales, UV, décodage du mip grossier. Clippy `--features dev --all-targets` propre.
- Exécution (debug, offscreen, `--state Campaign --frames 400`, M4 Pro) avec `enabled: true` : exit 0, aucune erreur
  shader/validation, proxy de 500 340 triangles construit en ~10 ms, albédo 885 x 763 décodé en ~0,4 s, dump
  `solari_campaign = {enabled, proxy_triangles 500340, camera_deferred true}`. Les nœuds du terrain sont masqués dès
  le départ (sans quoi le pipeline forward de `TerrainMaterial` est créé).
- `enabled: false` : exit 1, `Too many bindings of type Samplers ... count was 17` sur `opaque_mesh_pipeline`. PRÉEXISTANT
  (reproduit avec `solari.json` aussi coupé, cf. `bevy-sol2.md`) ; le matériau fautif est très probablement
  `TerrainMaterial` en forward (10 échantillonneurs propres) : avec Solari il n'est jamais dessiné et l'erreur disparaît.
  Conséquence : la base de comparaison `enabled:false` ne tourne pas tant que ce blocage n'est pas levé.
- Non vérifié : tout rendu (aucune capture), coût GPU, fidélité de l'albédo, bruit, comportement au zoom (rebuild).
- Le worktree n'a pas `data/map/pyramid` : lien symbolique local vers le dépôt principal (exclu de git).
