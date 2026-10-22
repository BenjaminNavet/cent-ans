# bevy-camp-samplers

Etat : FAIT (a verifier au merge). Campagne Metal : "Too many bindings of type Samplers ... 17".

- Avant : 7 samplers propres au terrain (colormap, ground, ab_dist, splat, terroir, coast, wx) + vue forward = 17.
- Apres : 3 (colormap, ground/bg_albedo, terroir), total 13. ab_dist, splat, coast_dist/probe/types et wx_field/clear
  lisent avec `terroir_sampler` : descripteurs identiques (linear, clamp-to-edge, sans mips/anisotropie, lectures
  textureSampleLevel mip 0) -> rendu strictement identique, aucun ecart de bord.
- colormap (mips + anisotropie 8, clamp) et ground (repeat + mips + anisotropie 8) gardent un vrai sampler.
- Test : `terrain_shader_owns_few_samplers` (material.rs) ; pas de test naga (terrain.wesl a trop d'imports Bevy a stubber).
- Verifie : campagne 300 images exit 0 sans erreur, bataille 200 images exit 0, clippy (defaut + dev), tests.
