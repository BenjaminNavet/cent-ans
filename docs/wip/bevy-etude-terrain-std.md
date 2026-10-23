# Étude : terrain de campagne sur base StandardMaterial ? (lecture seule, 22/10)

Aucune compilation ni lancement. [E] = établi par lecture du code, [S] = supposé / à mesurer. Chemins relatifs à `client/src/campaign/terrain/`.
Sources lues : `terrain.wesl` (2408 lignes), `material.rs`, `lod.rs`, `mesh.rs`, `solari_proxy.rs`, `post.wesl`, `relief_gpu.rs`, `bevy_pbr-0.20.0`, `docs/wip/bevy-sol2.md`, `bevy-solari-campagne-proto.md`, étude Solari campagne (branche a9ec5cac).

## Réponse courte
Non à `ExtendedMaterial<StandardMaterial, _>` complet. Il ne retire pas les 2 400 lignes (le look reste calculé dans le shader), mange 6 échantillonneurs pour rien et ne rend pas le terrain éligible à la TLAS de Solari (le proxy reste nécessaire). Recommandation : **option (a)** = garder `TerrainMaterial`, factoriser le fragment en « entrée PBR » et ajouter un chemin différé (`deferred_fragment_shader`) actif seulement sous Solari ; rendu par défaut (forward + MSAA 2x) inchangé.

## 1. Décomposition de terrain.wesl

Vertex (l. 262-325) : `displayed_y` (exagération verticale s, g, c ; relief dynamique sans reconstruire le maillage), géomorphe (l. 296-302, dépend de la distance caméra, par vertex et par image), enfoncement des bords (l. 307-312), `river_carve` (l. 278, texture `ab_dist` en vertex, fondu avec la distance caméra), normale analytique depuis `slope` (l. 318-324). Tout dépend de la vue ou de réglages dynamiques : n'est pas exprimable en géométrie « cuite ».

Fragment (l. 2283-2408). `P` = entrée PBR, `Q` = post-calcul.

| Bloc | Lignes | Ce que c'est | PBR / post |
|---|---|---|---|
| Colormap BC1 + masses sombres | 2285-2294, 1741 | albedo lointain | P base_color |
| Sol régional (45 couches, splat, biomes A/B, micro) | 1110-1192, 1781-1912, terroir 1297-1334 | albedo + dérivée de normale (`g.dn`) | P base_color + normal |
| Champs / parcelles peints | 1409-1716, 1913-2268 (~1000 lignes, le plus gros) | albedo, normale, couleur de saison | P base_color + normal |
| Détail proche | `ground_detail.wesl` (158), `gd_macro` | modulation d'albedo | P base_color |
| Canopée de forêt, sol sous forêt | 474, 2322-2330 | albedo + normale | P |
| Roche, neige, saisons, glaciers | 327-441, 619-649 | albedo (+ dépend de `lit0` = N.L x ombre) | P base_color, mais lit via `lit0` : terme d'éclairage injecté dans l'albedo (non physique) |
| Côte (falaises, plages, écume) | 650-786 | albedo + normale | P |
| Météo sol, brume au sol, voile de guerre | 530-618, 787-815 | albedo teinté | P base_color (mist/véil) |
| Ombres de cumulus | 836-867 | multiplie l'albedo | P (ou lumière : faux en différé car appliqué avant l'éclairage, voir 4) |
| Parchemin (vue stratégique) | 944-1105, 2358-2368 | albedo + émission 0,75 + normale aplatie | P base_color + emissive + normal |
| Occlusion de vallée `rvd` | 868-895 | multiplie l'ambiant seulement | P occlusion (diffuse_occlusion) |
| Éclairage maison | 2269-2281, 2370-2377 | `ambient*ao + sun*N.L*sun_vis`, ombres cascades `fetch_directional_shadow`, plancher d'ombre ciel (`shade_floor`, TS-T9) | Remplaçable par PBR (rugosité 1, réflectance 0 ≈ Lambert) sauf `shade_floor` (max non physique) = Q |
| `campaign_grade` | 442, 2379 | grade sur la couleur ÉCLAIRÉE, avant tonemap | Q (post), mais l'ordre grade -> AgX change le rendu |
| Brouillard distance + brume caméra (aérien par rayon, diffusion solaire) + bords de carte | 896-943, 2381-2399 | sur la couleur éclairée, `wx_aerial_fog_color` | Q ; `bevy_fog` du différé ne connaît pas la teinte par rayon ni la diffusion `wx_scatter` |
| Post maison | `post.wesl` | brume de vallée par altitude, perspective aérienne, grade final (après tonemap, lit la profondeur) | Q déjà séparé |

Bilan : environ 85 % du code (sol, champs, neige, côte, parchemin) produit albedo/normale/émission, donc va tel quel dans une entrée PBR. Les blocs qui exigent un travail sont les termes couplés à la lumière (`lit0` dans neige/roche, plancher d'ombre, grade pré-tonemap, brouillard par rayon). Roughness : aucune aujourd'hui (Lambert pur), donc constante.

## 2. Budget de liaisons (Metal : 16 échantillonneurs / étage fragment)

- `TerrainMaterial` aujourd'hui : **7 échantillonneurs propres** [E] (liaisons 2, 9, 12, 14, 25, 29, 33 de `material.rs` / `terrain.wesl` l. 41-236 ; le chiffre « 10 » de `bevy-sol2.md` est surévalué), 19 textures (1, 5, 8, 10, 11, 13, 15, 17, 18, 20, 21, 22, 24, 26, 28, 30, 31, 32, 34). Les textures ne limitent pas (la limite Metal est 128, 19 + vue passent déjà).
- Vue forward (`render/mesh_view_bindings.rs`) [E] : ombres 4 (l. 284, 287, 301, 304), LTC area_light 1 (l. 482), DFG 1 (l. 492), environnement 1 (l. 938/949), décals 1 (l. 1015) = **8**. Conditionnels : LUT de tonemapping +1 si `TONEMAP_IN_SHADER` (caméra LDR AgX de la campagne : probable, [S]), transmission +1 (l. 424), atmosphère +1 (l. 462). Donc 8 à 9 pour la campagne. Terrain actuel : 7 + 8..9 = **15 à 16**, déjà au plafond [S sur le +1].
- `StandardMaterial` : 6 échantillonneurs fixes (liaisons 2, 4, 6, 8, 10, 12 : base, émissif, métal-rugosité, normale, occlusion, profondeur) même avec les features `pbr_*` coupées ; chacun reste dans le layout même sans texture [E]. Avec les features (transmission 2 x3 textures, spéculaire 2, multi-couche 3, anisotropie 1) on irait au-delà de 6.
- `ExtendedMaterial<StandardMaterial, TerrainExtension>` : 6 + n propres + 8..9 vue ≤ 16, soit **n ≤ 2 (1 si LUT tonemap)**. Le terrain en a 7 : ne tient pas tel quel. Les liaisons de l'extension doivent aussi passer à 100+ (40 liaisons à renuméroter) [E, convention de `extended_material.rs`].
- Tenir sous la limite (chiffre exact) :
  1. Réutiliser les 6 échantillonneurs du Standard : assigner des images du terrain aux 6 slots de textures de base pour que leurs descripteurs d'image alimentent les 6 échantillonneurs, puis les lire par `base_color_sampler` etc. (0 propre, total 14 à 15). Hack fragile (les clés de spécialisation Standard lisent ces slots) [S].
  2. Ramener les 7 propres à 2 descripteurs distincts (répétition + aniso pour les tableaux ; linéaire-clamp pour toutes les cartes) : total 16 pile avec LUT tonemap = 0 marge. Plusieurs sont déjà identiques (commentaire `material.rs`).
  3. `textureLoad` + bilinéaire manuel : 4 taps par texture, trop cher pour les cartes chaudes (colormap, splat).
- Chemin différé : le view layout de la prépasse G-buffer ne porte ni ombres, ni LTC, ni DFG, ni env (lumière faite par la passe d'éclairage) [S : à vérifier à l'exécution]. Donc `TerrainMaterial` plain peut garder ses 7 échantillonneurs en différé sans toucher au plafond, à condition que le pipeline forward ne soit jamais créé (ce que fait déjà le proto : nœuds masqués).

## 3. Géométrie

- État actuel [E] : une entité par nœud (`lod.rs` l. 356-369), un `Mesh3d` CPU par nœud (`mesh.rs`, 64x64 quads + jupe, ~8,7 k tri), **un seul `Handle<TerrainMaterial>` partagé** (`lod.material`, `MeshMaterial3d(material.clone())`), `Aabb` explicite + `NoFrustumCulling` (sélection par le quadtree). Le matériau est donc déjà partagé et sans uniformes par nœud : tout par sommet. Rien à gagner côté `MeshTag` pour les réglages ; le coût `check_entities_needing_specialization` vient du nombre d'entités (350-900), pas des matériaux [S, à mesurer].
- Déplacement au vertex shader (statu quo) : garde exagération dynamique et géomorphe sans reconstruire. Maillage pré-déplacé : perd cela (rebuild de tous les nœuds à chaque changement d'exagération, popping à la transition de LOD) ; seul le BLAS Solari en a besoin, d'où le proxy de `solari_proxy.rs` (grille ~500 k tri sur `display_height_at`, rebuild si l'échelle varie de 5 %, au plus toutes les 1,5 s). Géomorphe et rivières creusées absents du proxy : acceptable pour du RT à cette échelle.
- Solari [E] : seuls `RaytracingMesh3d` + `MeshMaterial3d<StandardMaterial>` entrent dans la TLAS ; un `ExtendedMaterial` n'y entre pas. Donc (b) n'ouvre pas Solari : le proxy reste. Piste [S] : le proxy peut ne servir qu'aux rayons (invisible au raster) pendant que les nœuds visibles écrivent le G-buffer ; non établi (visibilité vs extraction RT à vérifier).
- Réduire les draws : chaque nœud = 1 draw par phase (principale + cascades d'ombre + prépasse). Un patch partagé unique + `MeshTag`/transform par instance serait batché par Bevy en 1 draw/phase [S], hauteurs/pentes/champs lus en vertex depuis des pages GPU. Le relief global existe déjà côté GPU (`relief_gpu.rs`, texture floor/base/k/D) mais pas les hauteurs fines (pyramide CPU). C'est l'option (c) : gros chantier, indépendant du choix standard/maison. Indirect/GPU culling Bevy : multi-draw indirect indisponible sur Metal [S], gain limité, et le culling GPU suppose l'Aabb correct (déjà explicite).

## 4. Différé

- G-buffer Bevy [S d'après `bevy-sol2.md` S3] : base_color 8 bits sRGB, rugosité 8 bits, normale octaédrique, émissif rgb9e5, occlusion/réflectance 8 bits. Tout ce que `terrain.wesl` produit (albedo, normale, émission parchemin) y passe, avec perte de précision (bandes dans les sols sombres).
- Perdu : éclairage maison (plancher d'ombre TS-T9, grade avant tonemap, ordre brouillard/grade), brouillard par rayon et diffusion solaire de `camera_distance_fog` (à refaire en post depuis la profondeur, `post.wesl` fait déjà l'aérien), ombres de cumulus (appliquées à l'albedo : en différé elles resteraient valides tant qu'elles sont dans `base_color`, mais ne dépendent plus de l'ombre solaire).
- Prépasse : en différé, profondeur + normales fournies ; le post maison lit la profondeur en variante multisample (`post.wesl` l. 51-53) : variante simple si MSAA off. Ombres : `terrain_shadow.wesl` (déplacement identique) reste le caster ; sous Solari les cascades sont remplacées par les rayons (proto).
- MSAA : le différé impose `Msaa::Off` (Solari aussi), la campagne est en 2x. Perte d'anticrénelage sur le relief à haute fréquence : gérée par SMAA/TAA à évaluer, risque visuel réel. D'où : différé réservé au mode Solari.

## 5. Options

| | Effort | Risque visuel | Gain |
|---|---|---|---|
| (a) Matériau maison + chemin différé (sous Solari) | moyen, ~3 lots | nul par défaut (forward intact) ; différé : écarts de look acceptés car mode expérimental | G-buffer pour Solari, un seul fragment pour les 2 chemins ; code inchangé ; draws inchangés |
| (b) `ExtendedMaterial<Standard, Ext>` complet | gros : renumérotation 100+, 2 descripteurs d'échantillonneurs ou hack des 6 slots, uniforme Standard, clés de spécialisation, mêmes blocs P/Q qu'en (a) | moyen (éclairage Standard à calibrer : unités lux/exposition, spéculaire GGX à neutraliser) | « moins de code » illusoire : 85 % du shader reste ; pas d'entrée TLAS ; 0 de marge d'échantillonneurs |
| (c) Patch partagé instancié + pages GPU (+ clipmap) | très gros : nouveau maillage GPU, pages de hauteur GPU, normales/jupes/géomorphe en VS, adaptation de `surface.rs`, rivières, ombres | élevé (géométrie du rendu identitaire) | réduit draws (700 -> ~1/phase) et spécialisation CPU ; indépendant de Solari |

Recommandation : **(a)**, (c) seulement si le banc montre le CPU/draws dominants.

Lots de (a), chacun mesuré par `tools/bevy_bench/bench.sh <étiquette> --offscreen` (base `terr_base` avant tout changement) :
1. Mesure de départ : `terr_base` (ms GPU/CPU, draws, temps de `check_entities_needing_specialization`) ; compter exactement les échantillonneurs de la vue campagne (tonemap in shader ou non) par un test/log.
2. Refonte `fragment` : sortie scindée en `terrain_surface(in) -> {albedo, normal, ao, emissive}` (pur déplacement de code, forward identique octet pour octet ; test naga existant) + `lighting_forward`. Bench `terr_refactor` = neutre.
3. `deferred_fragment_shader` pour `TerrainMaterial` (PbrInput rempli à la main, rugosité 1, réflectance 0, `deferred_output`) ; brume/grade déplacés en post pour ce chemin ; combiner avec `solari_proxy.rs` (nœuds visibles écrivent le G-buffer, proxy pour les rayons). Bench `terr_def` vs `terr_base` (ms passe, mémoire BLAS, rebuild en mouvement) à d = 20, 60, 150. Critère d'arrêt déjà posé : < 4 ms sur M4 Pro.
4. (conditionnel) (c) seulement si lot 1 montre un CPU de rendu dominant.

## Points ouverts
- Veut-on Solari en campagne malgré le bruit sans débruiteur Apple ? (arbitrage utilisateur, pas tranché ici ; l'étude précédente recommande d'abord ombre de relief ray-marchée + AO d'horizon.)
- Vérifier à l'exécution : échantillonneurs de la vue campagne (8 ou 9), view layout allégé du différé, extraction RT d'un mesh invisible.
- Écart : `bevy-sol2.md` compte 10 échantillonneurs propres au terrain ; j'en lis 7.
