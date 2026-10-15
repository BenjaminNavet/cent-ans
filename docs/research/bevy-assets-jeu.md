# Survol de l'écosystème Bevy pour Cent Ans (15/10/2026)

Méthode : API crates.io (versions, dépendances `bevy`, licences, téléchargements récents), mots-clés du brief. Le client est sur **Bevy =0.20.0** (sorti le 08/10/2026) avec seulement `bevy`, `png`, `flate2`, `serde` : tout le rendu campagne/bataille est donc fait main. Beaucoup de crates sont encore en 0.19 (une version de retard) : « 0.19 » = port à attendre ou à forker, pas un blocage ferme. Les pages bevy.org/assets n'ont pas été parcourues une à une ; la sélection vient de crates.io. Les fonctions intégrées à Bevy 0.20 (atmosphère, brouillard volumétrique, décalques, instanciation GPU) n'ont pas été vérifiées dans le code : à contrôler avant d'écrire du custom.

Légende ajustement : **adopt** (prendre), **try** (spike court), **watch** (surveiller), **skip**. Licences : toutes MIT/Apache-2.0 sauf mention.

## 3D / terrain
| Crate | Apport | Bevy / dernière maj | Maturité | Fit |
|---|---|---|---|---|
| bevy_heightmap 0.19.0 | maillage depuis PNG heightmap | 0.19 / 06-2026 | 211 dl récents | skip (relief-lod maison est plus avancé : LOD, texture arrays) |
| bevy_symbios_ground 0.7 | terrain algorithmique | 0.19? / 10-2026 | 414 dl, confidentiel | skip |
| bevy_3d_tiles 0.5.1 | streaming OGC 3D Tiles | 0.19 / 10-2026 | 754 dl | watch (idée : tuiles streamées si la carte grossit) |
| big_space 0.14.0-rc.0 | origine flottante, grandes coordonnées | 0.20-rc / 10-2026 | 38k dl | watch (utile si précision f32 gêne sur l'Europe entière) |
| noisy_bevy 0.14 (MIT) | bruit (simplex, fbm) côté WGSL/CPU | 0.19 / 06-2026 | 93k dl | try (shaders de bruit prêts à l'emploi) |
| bevy_gltf_draco 0.2.1 | glb compressés Draco | 0.19 / 09-2026 | 667 dl | watch (poids des modèles villes/châteaux) |
| bevy_materialize 0.12 | matériaux typés chargés depuis fichiers | 0.20 / 10-2026 | 33k dl | try (données `data/` -> matériaux) |
| bevy_polyline 0.14.1 | lignes épaisses GPU (frontières, routes, rivières) | 0.18 / 10-2026 | 238k dl | try (frontières de factions, itinéraires) ; port 0.20 à vérifier |
| bevy_mod_outline 0.13 / bevy_mesh_outline 0.4.2 | contours de sélection | 0.19 / 07 et 10-2026 | 117k / 1k dl | try (surbrillance armées/villes) |

Aucun clipmap ni terrain LOD générique actif et compatible n'est ressorti : le terrain maison reste justifié.

## Végétation
| Crate | Apport | Bevy / maj | Maturité | Fit |
|---|---|---|---|---|
| bevy_feronia 0.8.4 | scattering herbe/feuillage + vent | 0.18 / 06-2026 | 3k dl | watch (inspirer les shaders de vent, pas à adopter) |
| bevy_eidolon 0.5.0 | matériau instancié générique | 0.19 / 08-2026 | 2k dl | watch (comparer avec notre instanciation maison) |
| bevy_flora 0.46 / bevy_procedural_tree 0.4 | arbres paramétriques, impostors | 0.19-0.20 / 10-2026 | 758 / 796 dl | skip (nos arbres viennent de modèles/Meshy) |
| warbler_grass 0.6.1 | herbe GPU | 0.13 (abandonné) | - | skip |

Verdict : pas de solution mûre ; garder le crate `vegetation` et notre instanciation.

## Eau / ciel / météo
| Crate | Apport | Bevy / maj | Maturité | Fit |
|---|---|---|---|---|
| bevy_weather 0.2.0 | cycle jour/nuit, ciel, météo | 0.19.1 / 09-2026 | 57 dl | watch (jeune, réf. d'architecture) |
| bevy_elements 0.5.0 | soleil/lune/étoiles, météo climatique | **0.20** / 10-2026 | 21 dl | watch |
| bevy_sky_gradient 0.4 | dégradé de ciel + aurores | 0.19 / 08-2026 | 494 dl | skip |
| bevy_aqua 0.1.3 | océan Gerstner/FFT autour de la caméra | 0.19 / 08-2026 | 99 dl | watch (mers/Méditerranée si on veut mieux que l'eau actuelle) |
| bevy_water 0.18.1 | eau simple | 0.18 / 02-2026 | 4k dl | skip |
| bevy_atmosphere 0.13 | ciel procédural | 0.16 (périmé) | - | skip (Bevy a une atmosphère intégrée à vérifier) |
| bevy_volumetric_clouds 0.2 (MIT) | nuages volumétriques | 0.18 / 01-2026 | 133 dl | skip |

## Animation / foules
Rien d'actif ne couvre l'animation par texture de sommets ni le skinning instancié de milliers de soldats (recherches « vertex animation », « crowd », « gpu skinning » : seulement des avatars paramétriques et du gore). À garder en maison ; surveiller les notes de version de Bevy (skinning GPU, instanciation) plutôt qu'un crate. `bevy_tweening 0.16` (0.19, 179k dl) pour les animations d'UI et de caméra : try.

## Particules
| Crate | Apport | Bevy / maj | Maturité | Fit |
|---|---|---|---|---|
| bevy_hanabi 0.19.0 | particules GPU (feu, fumée, poussière, trainées) | 0.19 / 06-2026 | 183k dl, 28k récents, réf. de fait | **try -> adopt** dès sortie du port 0.20 (ou fork local) |
| bevy_firework 0.10 | particules GPU plus simples | 0.19 / 07-2026 | 377 dl | skip (hanabi) |
| bevy_enoki 0.8 (MIT) | particules 2D | **0.20** / 10-2026 | 30k dl | try (vue parchemin 2D) |

## Audio
| Crate | Apport | Bevy / maj | Maturité | Fit |
|---|---|---|---|---|
| bevy_kira_audio 0.27.1 | Kira : canaux, fondus, séquences, spatial simple | **0.20** / 13-10-2026 | 545k dl | **adopt** (playlists musique, ambiances, fondus ; aligné 0.20) |
| bevy_seedling 0.8 | Firewheel : graphe audio, spatial, HRTF (firewheel-ircam-hrtf) | 0.19 / 08-2026 | 31k dl | watch (plus ambitieux pour le spatial des batailles ; port 0.20 à attendre) |
| audionimbus 0.16 | Steam Audio (licence Apache pour le wrapper, SDK à vérifier) | - | 1.7k dl | skip |

## UI
| Crate | Apport | Bevy / maj | Maturité | Fit |
|---|---|---|---|---|
| bevy_ui_widgets 0.20 / bevy_feathers 0.20 | widgets officiels sans style / style éditeur | **0.20** (dans Bevy) | 1M dl | try (base boutons, sliders, menus de réglages sans dépendance) |
| bevy_egui 0.43.1 (MIT) | egui dans Bevy | **0.20** / 10-2026 | 2,7M dl | adopt pour outils dev/debug, pas pour l'UI joueur |
| bevy-inspector-egui 0.37 | inspecteur ECS | 0.19 / 06-2026 | 1,5M dl | try dès port 0.20 (dev) |
| pyri_tooltip 0.7 | infobulles | 0.19 / 08-2026 | 54k dl | try (cf. besoin d'infobulles type BG3 ; port 0.20 attendu) |
| bevy_nested_tooltips 0.10 (MIT) | infobulles imbriquées (chaînes Alt) | **0.20** / 10-2026 | 248 dl | try (colle à nos « Alt chains ») |
| bevy_fluent 0.15 | localisation Fluent | 0.19 / 06-2026 | 6k dl | try (FR/EN, fichiers .ftl) |
| bevy_lunex 0.7, bevy_cobweb_ui 0.22 | UI layout alternative | 0.19 et 0.18 | peu actifs | skip |
| bevy_vello 0.14 | vecteur 2D GPU (Vello) | 0.19 / 08-2026 | 34k dl | watch (parchemin vectoriel) |
| bevy_vector_shapes 0.14 | formes 2D/3D immédiates | **0.20** / 10-2026 | 152k dl | try (cercles de portée, zones, debug) |
| bevy_prototype_lyon 0.17 | tracés 2D | 0.19 | 373k dl | skip |

## Caméra / picking / entrées
| Crate | Apport | Bevy / maj | Maturité | Fit |
|---|---|---|---|---|
| bevy_rts_camera 0.15 | caméra RTS (pan, zoom, rotation, suit le terrain) | **0.20** / 14-10-2026 | 36k dl | **try** (comparer avec notre caméra de campagne/bataille ; hauteur sur terrain) |
| bevy_panorbit_camera 0.35.1 | orbitale | 0.19 / 09-2026 | 362k dl | try (outils, galeries) |
| bevy_pancam 0.21.1 | pan/zoom 2D | 0.19 / 09-2026 | 108k dl | try (vue parchemin) |
| bevy_enhanced_input 0.27 | entrées par contextes, rebinding | **0.20** / 14-10-2026 | 182k dl | **adopt** (raccourcis modifiables, contextes campagne/bataille/menu) |
| leafwing-input-manager 0.21 | actions d'entrée | 0.19 / 06-2026 | 577k dl | alternative (même rôle) ; choisir un seul |
| bevy_mod_picking 0.20.1 | picking | périmé (picking intégré à Bevy depuis 0.15) | - | skip |
| bevy_transform_interpolation 0.5 | interpolation fixe -> rendu | 0.19 / 06-2026 | 476k dl | try (mouvement fluide des armées si sim à pas fixe) |
| bevy_framepace 0.22 | limiteur de FPS | 0.19 / 07-2026 | 470k dl | try (économie CPU, dev) |

## IA / pathfinding / physique / sauvegardes
| Crate | Apport | Bevy / maj | Maturité | Fit |
|---|---|---|---|---|
| polyanya 0.17.1 / vleue_navigator 0.16 | navmesh any-angle (pur Rust pour polyanya) | polyanya indépendant / vleue 0.19.1 | 99k / 66k dl | watch : les règles vivent dans `core/` ; seulement pour le rendu (aperçu de trajet) |
| bevy_rerecast 0.5 + landmass 0.9.3 | navmesh Recast + évitement local d'agents | 0.19 / 08-2026 | 18k / 32k dl | watch (batailles : évitement ; si besoin côté `core/`, landmass est agnostique de Bevy) |
| bevy_flowfield_tiles_plugin 0.15 | flowfield par tuiles | 0.19 / 08-2026 | 20k dl | watch (foules en bataille ; décision sim = core) |
| bevy_northstar 0.7 (MIT) | A* hiérarchique grille | 0.19 | 10k dl | skip |
| hexx 0.25 (Apache-2.0) | utilitaires hexagonaux | bevy_ecs 0.19 | 168k dl | skip (cartes par provinces, pas d'hex) |
| avian3d 0.7 / bevy_rapier3d 0.37 (Apache-2.0) | physique 3D | 0.19 / **0.20** | 405k / 603k dl | skip (pas de physique de jeu ; la sim est dans `core/`) |
| moonshine-save 0.8 (MIT) | sauvegarde ECS par composants | **0.20** / 11-10-2026 | 45k dl | skip : l'état de partie est déjà sérialisé par `core/` ; garder serde |
| bevy_pkv 0.16 | préférences clé/valeur | 0.19 / 06-2026 | 95k dl | try (réglages, raccourcis) ; sinon serde_json maison |
| bevy_replicon 0.45.1 | réplication réseau | **0.20** / 13-10-2026 | 216k dl | skip (solo) ; watch si multijoueur un jour |

## Outils / dev / tests agents
| Crate | Apport | Bevy / maj | Maturité | Fit |
|---|---|---|---|---|
| bevy_brp_extras 0.23 | méthodes BRP en plus : **capture d'écran**, arrêt, simulation de touches | **0.20** / 09-10-2026 | 15k dl | **adopt** : complète notre feature `remote` (bevy_remote) pour les captures et les tests pilotés par agent |
| bevy_capture 0.6 | enregistrement d'images/GIF | 0.19 / 06-2026 | 469 dl | watch |
| bevy_asset_loader 0.28.1 | chargement d'assets par états, collections | **0.20** / 13-10-2026 | 662k dl | **adopt** (écrans de chargement, collections typées) |
| bevy_common_assets 0.18 | chargement RON/JSON/YAML/TOML comme assets | **0.20** / 09-10-2026 | 342k dl | **adopt** si les données `data/` passent par l'AssetServer (rechargement à chaud) |
| bevy_screen_diagnostics 0.8 / iyes_perf_ui 0.5 | HUD de perfs | 0.16 et plus ancien | périmé | skip (utiliser l'egui + diagnostics Bevy) |
| bevy_mod_scripting 0.23 | scripts Lua/Rhai | **0.20** / 14-10-2026 | 49k dl | skip (modding plus tard) |
| bevy_infinite_grid 0.18 | grille de repère | 0.18 | 399k dl | skip |
| tracy / chrome trace | profilage | intégré à Bevy | - | déjà couvert par la feature `profile` |

## Top 10 recommandations
1. **bevy_kira_audio 0.27.1** (0.20) : adopter pour musique à playlists, ambiances, fondus ; gros gain contre du code audio maison.
2. **bevy_enhanced_input 0.27** (0.20) : adopter pour raccourcis rebindables et contextes (campagne/bataille/menus), prérequis des réglages joueur.
3. **bevy_brp_extras 0.23** (0.20) : adopter pour captures d'écran et simulation d'entrées via BRP, au service des tests pilotés par agent (sans voler le focus).
4. **bevy_asset_loader 0.28.1** + **bevy_common_assets 0.18** (0.20) : adopter pour états de chargement et données `data/` en assets rechargeables à chaud.
5. **bevy_hanabi 0.19** : particules GPU feu/fumée/poussière ; attendre ou forker le port 0.20, c'est la référence du domaine.
6. **bevy_egui 0.43.1** (0.20) : outils de dev et debug seulement, pas l'UI joueur.
7. **bevy_rts_camera 0.15** (0.20) : spike de comparaison avec notre caméra, utile pour hauteur-sur-terrain et bords.
8. **bevy_nested_tooltips 0.10** (0.20) et/ou **pyri_tooltip 0.7** (0.19) : infobulles imbriquées style BG3 ; plus **bevy_ui_widgets/feathers 0.20** pour les widgets de base.
9. **bevy_polyline 0.14.1** + **bevy_mod_outline 0.13** : frontières/routes en lignes GPU et contours de sélection (ports 0.20 à vérifier).
10. **bevy_fluent 0.15** (localisation) et **bevy_transform_interpolation 0.5** (fluidité des armées) : à essayer après port 0.20.

Constats négatifs utiles : pas de crate mûr pour terrain LOD/clipmap, herbe/végétation instanciée, animation par texture de sommets, foules skinnées, nuages ou météo ; le travail maison (relief-lod, vegetation, instanciation des soldats) reste justifié. Navigation, physique et sauvegarde : les règles sont dans `core/`, donc skip. Aucune licence non MIT/Apache repérée (bevy_rapier3d et hexx sont Apache-2.0 seul, bevy_egui/noisy_bevy/moonshine/bevy_northstar/bevy_enoki MIT seul).
