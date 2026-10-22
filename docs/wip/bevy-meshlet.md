# Bevy 0.20 — essai `meshlet` (lot meshlet)

État : analyse de faisabilité terminée par lecture du code (bevy_pbr 0.20.0 `src/meshlet`, wgpu-hal 30.0.1) ; aucune compilation, aucun prototype. Verdict : NON FAISABLE pour le décor dense, on s'arrête.

## Verdict : incompatible

Contraintes de Bevy (doc de `MeshletMesh`) face au code du dépôt :

| Contrainte Bevy | Notre décor | Compatible |
|---|---|---|
| Matériaux opaques uniquement (pas d'alpha mask) | Feuillage = cartes `AlphaMode::Mask(0.5)` (`battle/decor/trees.rs:674`), herbe `Mask(0.4)` (`grass.rs`), lointain `far_woods.rs`, sous-bois et touffes de campagne `Mask` (`campaign/countryside/understory_layer.rs`, `clutter.rs`) | non |
| Aucun contrôle du vertex shader | Balancement au vent via `ExtendedMaterial<StandardMaterial, WindExtension>` à vertex shader (`battle/decor/wind.rs`, `wind.wesl`) | non |
| Pas de `diffuse_transmission`, double face, dérivées auto | Feuilles : `diffuse_transmission`, `double_sided` | non |
| Fragment via `meshlet_mesh_fragment_shader` (motifs shader spécifiques) | Extensions de matériaux écrites pour le chemin classique | à réécrire |
| MSAA obligatoirement `Off` | Presets de qualité avec `msaa` (`settings/quality.rs`) | partiel |
| Pas de mélange forward+prepass / deferred | à vérifier, non testé | inconnu |
| Meshes pré-traités (`MeshletMesh`, feature `meshlet_processor`, lent) | nos arbres sont générés procéduralement par code (`tree_kit`), pas des assets ; il faudrait un pré-traitement hors ligne ou au démarrage | coût élevé |
| Benéficie aux meshes très haute densité de triangles | arbres = cartes de feuilles (quelques centaines de triangles), l'instancing automatique de Bevy fait déjà le travail ; surcoût de base meshlet « plus lent avec peu de géométrie » | gain nul voire négatif |

Seule pièce compatible : l'écorce (`StandardMaterial` opaque, normal map). Mais troncs minuscules (quelques dizaines de triangles), donc aucun gain, et le coût (plugin, feature GPU, MSAA Off, deux chemins de rendu) n'est pas justifié.

## Matériel

Le GPU local (Apple M4 Pro, Metal 4) expose bien `TEXTURE_INT64_ATOMIC` (wgpu-hal metal/adapter.rs), donc le plugin démarrerait ici ; le blocage est le matériau, pas le GPU. Le plugin n'est supporté que Vulkan/Metal (pas DX12), ce qui exclurait le portage Windows par défaut.

## Coût de mise en place si on insistait

Remplacer le feuillage par de la vraie géométrie opaque haute densité (feuilles modélisées, plus de cartes alpha), réécrire le vent hors vertex shader (impossible : pas de contrôle des sommets), préprocesser chaque espèce en `MeshletMesh`, retirer MSAA. Équivaut à refaire la chaîne d'arbres ; hors enveloppe du lot.

## Livrables

- Feature Cargo `meshlet` : non ajoutée (pas de prototype, pas de bloc dans `client/Cargo.toml`).
- Mesure au banc : sans objet.
- Prochaine étape : aucune. Rouvrir uniquement si un jour le décor passe à de la géométrie opaque très dense (roches, bâtiments en pierre) : candidats éventuels `battle/siege/stone.rs`, `campaign/countryside/rocks.rs` (déjà `Opaque`), à évaluer séparément.
