// Arbres proches de la carte en cartes de feuillage (lot FC5) : même corps que `foliage.gdshader`
// (`foliage.gdshaderinc`) avec FOLIAGE_CARDS : albédo modulé par l'atlas des cartes
// (`campaign_leaf_cards.png`, UV2), découpe alpha, cartes vues des deux côtés (normale arrondie
// du houppier gardée de chaque côté). Le découpage fait déjà un discard : l'hiver ajouré
// (FOLIAGE_WINTER_CUTOUT) est toujours compilé, sans variante de saison.
shader_type spatial;
render_mode diffuse_lambert_wrap, specular_schlick_ggx, cull_disabled;

#define FOLIAGE_CARDS
#define FOLIAGE_WINTER_CUTOUT
#include "res://shaders/foliage.gdshaderinc"
