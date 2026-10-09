# Index des décisions d'architecture

Généré par `tools/adr_index.py` (ne pas éditer à la main). « bis » : numéro attribué deux fois, fichiers non renommés.
Prochain numéro libre : 0193 (0200-0209 réservés au chantier SC).

| N° | Titre | Statut |
|---|---|---|
| 0001 | [Cœur de jeu headless en Rust, exposé à Godot via GDExtension](0001-headless-core-rust-gdextension.md) | accepté |
| 0002 | [Un fichier par jalon dans le pont et l'interface](0002-one-file-per-milestone-bridge-and-ui.md) | accepté |
| 0003 | [IA stratégique dans la crate `ai`, IA minimale dans la simulation](0003-strategic-ai-in-ai-crate.md) | accepté |
| 0004 | [Direction artistique semi-réaliste](0004-semi-realistic-art-direction.md) | accepté |
| 0005 | [Colonies prenables à l'intérieur des provinces](0005-settlements-within-provinces.md) | accepté |
| 0006 | [Figurines de bataille modelées sous Blender, animées par shader](0006-battle-figures-from-blender.md) | n/d |
| 0007 | [Physique Godot réservée au rendu (Jolt), jamais remontée au cœur](0007-physique-rendu-seulement.md) | n/d |
| 0008 | [Les incendies de siège sont une règle du cœur](0008-incendies-regle-coeur.md) | n/d |
| 0009 | [Agents de campagne : entités légères hors dynastie](0009-agents.md) | accepté |
| 0010 | [Mouvement libre des armées](0010-free-army-movement.md) | accepté |
| 0011 | [Édits régionaux et chaînes de bâtiments (lot C4 « Total War »)](0011-edicts-and-building-chains.md) | accepté |
| 0012 | [Routes commerciales et accords : dérivées du graphe, pas d'état de rupture](0012-trade-routes.md) | accepté |
| 0013 | [G1 : auto-résolution par phases, calibrée sur la bataille 3D](0013-g1-auto-resolution-par-phases.md) | accepté |
| 0014 | [Figurines de bataille skinnées, animations cuites en texture d'os](0014-figurines-skinnees-texture-os.md) | n/d |
| 0015 | [Villes emblématiques : plan en données, maquette générée, loupe radiale](0015-landmark-cities.md) | accepté |
| 0016 | [Taille des unités : multiplicateur visuel de figurines](0016-taille-des-unites-visuelle.md) | accepté |
| 0017 | [Préréglages de qualité du rendu, ciels HDRI et étalonnage par données](0017-qualite-rendu-et-atmosphere.md) | accepté |
| 0019 | [Relief fin et occupation du sol historique de la carte de campagne](0019-relief-et-occupation-du-sol.md) | accepté |
| 0020 | [Relief des champs de bataille](0020-relief-des-champs-de-bataille.md) | accepté |
| 0021 | [Kit de bâtiments réalistes (carte de campagne et batailles)](0021-kit-batiments-realistes.md) | accepté |
| 0022 | [Chocs de cavalerie, morts, sang et démembrements : règles au cœur, rendu par instances](0022-chocs-morts-et-sang-rendus.md) | n/d |
| 0023 | [Assaut de siège : événements de rendu du cœur et grimpeurs posés par le cœur](0023-evenements-assaut-de-siege.md) | accepté |
| 0024 | [Imposteurs lointains des figurines, cuits en jeu depuis les figurines V2](0024-imposteurs-lointains-des-figurines.md) | n/d |
| 0025 | [Négociation à plusieurs clauses, buts de guerre et fatigue de guerre (lot DP1)](0025-negociation-et-buts-de-guerre.md) | accepté |
| 0026 | [Sièges dans le plan des villes emblématiques](0026-sieges-dans-le-plan-des-villes-emblematiques.md) | accepté |
| 0027 | [Météo de la carte de campagne : tirée au cœur, déterministe, visuelle pour l'instant](0027-meteo-de-campagne.md) | n/d |
| 0028 | [Batailles navales](0028-batailles-navales.md) | partiellement remplacé par l’ADR 0201 |
| 0029 | [Révoltes à délai, troubles de province et coûts d'événements proportionnels](0029-revoltes-et-couts-d-evenements.md) | accepté |
| 0031 | [Dossier utilisateur propre au jeu exporté](0031-dossier-utilisateur-du-jeu-exporte.md) | n/d |
| 0032 | [Horizon des batailles : relief réel lointain et panoramas peints](0032-horizon-de-bataille.md) | accepté |
| 0033 | [Eau, ponts et routes du champ de bataille](0033-eau-ponts-routes.md) | accepté |
| 0034 | [Porte-étendards, musiciens, étendards tombés et pris](0034-porte-etendards.md) | n/d |
| 0035 | [Cartes de batailles historiques (Crécy, Poitiers, Azincourt)](0035-cartes-historiques.md) | accepté |
| 0036 | [Carte de campagne zoomable : pyramide de relief streamée jusqu'à 1-5 m](0036-carte-de-campagne-zoomable-relief-streame.md) | accepté |
| 0037 | [Niveaux de difficulté de la campagne](0037-niveaux-de-difficulte.md) | accepté |
| 0045 | [Forêts historiques dans la grille de navigation](0045-forets-historiques-dans-la-grille.md) | accepté |
| 0046 | [Contre-pente contre l'arc long, position « couvert × crête »](0046-contre-pente-et-couvert-crete.md) | accepté |
| 0047 | [Ville de siège dense et mobilier de rue solide](0047-ville-de-siege-dense-et-mobilier-solide.md) | accepté |
| 0048 | [Filtres de la carte de campagne](0048-filtres-de-carte.md) | accepté |
| 0049 | [Impôt de guerre sur la réserve, troubles qui retombent, un palier par chaîne au départ](0049-equilibre-eq2-impot-troubles-paliers.md) | accepté |
| 0050 | [Interface « manuscrit enluminé » : kit de textures 9-slice procédural](0050-interface-enluminee-kit-9-slice.md) | accepté |
| 0051 | [Budget de temps par image sur la carte, calculs lourds hors du fil principal](0051-budget-image-carte-et-calculs-hors-fil.md) | n/d |
| 0052 | [Panique des chevaux sous les traits, haie tenue par les tireurs](0052-panique-des-chevaux-sous-les-traits.md) | accepté |
| 0053 | [Bâtiments : améliorations sans régression, `enables_units`, ressources de chantier](0053-batiments-ameliorations-unites-ressources.md) | n/d |
| 0054 | [Une trêve lie les alliés entrés dans la guerre (lot EQ3)](0054-treve-liant-les-allies.md) | accepté |
| 0055 | [Mise en scène des batailles : heure du jour, nuages, fumées, oiseaux, plan cinématique](0055-mise-en-scene-des-batailles.md) | accepté |
| 0056 | [Batailles décisives : armée brisée, bataille refusée](0056-batailles-decisives-armee-brisee-bataille-refusee.md) | accepté |
| 0060 | [Musique d'époque libre de droits et bataille en couches](0060-musique-d-epoque.md) | accepté |
| 0061 | [Villages et décor du champ de bataille](0061-villages-et-decor-du-champ-de-bataille.md) | accepté |
| 0062 | [Semis natif de la végétation de campagne (PB2)](0062-semis-natif-de-la-vegetation.md) | accepté |
| 0063 | [Portraits vivants (lot DA2)](0063-portraits-vivants.md) | accepté |
| 0064 | [Armoiries des maisons et héraldique des figurines (DA1)](0064-armoiries-des-maisons.md) | accepté |
| 0065 | [Boutons-médaillons enluminés et icônes d'action à l'encre (DA5)](0065-boutons-et-icones-enlumines.md) | n/d |
| 0066 | [Marqueurs de carte : un seul langage (DA3)](0066-marqueurs-de-carte.md) | n/d |
| 0067 | [Végétation de bataille : lisières douces, feuillus ramifiés, imposteurs (DA6)](0067-vegetation-de-bataille.md) | accepté |
| 0068 | [Direction de la déroute et contagion de moral](0068-direction-de-la-deroute-et-contagion.md) | accepté |
| 0069 | [L'IA budgète ses engagements et ne campe plus en terre étrangère (lot EQ5)](0069-ia-budget-engagements-et-retour-au-pays.md) | accepté |
| 0070 | [Blessés au sol et fuyards désarmés (lot EP12)](0070-blesses-au-sol-et-fuyards-desarmes.md) | accepté |
| 0071 | [Poussée continue des lignes en mêlée](0071-poussee-continue-des-lignes.md) | accepté |
| 0072 | [Rejeu d'après bataille](0072-rejeu-d-apres-bataille.md) | accepté |
| 0073 | [Relecture du tour de l'IA : enregistrement des marches dans le cœur](0073-relecture-du-tour-ia.md) | accepté |
| 0074 | [Frontières de faction lumineuses peintes dans le fragment du terrain (lot FR1)](0074-frontieres-de-faction.md) | accepté |
| 0075 | [Droit de passage, carte diplomatique et refus expliqués (lot DP2)](0075-droit-de-passage-et-carte-diplomatique.md) | accepté |
| 0076 | [Échelle massive des batailles](0076-echelle-massive.md) | accepté |
| 0077 | [Hébergement du relief fin « Cent Ans relief »](0077-hebergement-du-relief-fin.md) | accepté |
| 0078 | [Villes emblématiques à l'échelle 1:1 géoréférencées (format `landmark` v2)](0078-villes-emblematiques-1-1.md) | accepté |
| 0079 | [Profil de build Rust (Apple Silicon)](0079-profil-de-build-rust.md) | accepté |
| 0080 | [Mise à l'échelle 3D MetalFX](0080-mise-a-l-echelle-metalfx.md) | accepté |
| 0081 | [Fin de tour dans un fil](0081-fin-de-tour-dans-un-fil.md) | accepté |
| 0082 | [Carte plus dense et plus lente (DC)](0082-carte-plus-dense-et-plus-lente.md) | accepté |
| 0083 | [La cavalerie de l'assaillant attend son infanterie sous les flèches](0083-la-cavalerie-attend-son-infanterie.md) | accepté |
| 0084 | [Pas de compilation incrémentale en profil dev](0084-pas-de-compilation-incrementale.md) | accepté |
| 0085 | [La guerre de Cent Ans à tous les niveaux de difficulté (lot EQ6)](0085-guerre-de-cent-ans-a-toutes-difficultes.md) | accepté |
| 0086 | [Rasters de la carte : pixel i centré en x = i + 0,5 (comme les outils)](0086-convention-demi-pixel-des-rasters.md) | accepté |
| 0087 | [Portage Windows x86_64](0087-portage-windows.md) | accepté |
| 0088 | [Matières cuites des figurines fines (lot FG3)](0088-matieres-cuites-des-figurines-fines.md) | accepté |
| 0089 | [Figurines fines par défaut, LOD0 par soldat (lot FG5)](0089-figurines-fines-par-defaut.md) | accepté |
| 0090 | [Pas de bataille dans un fil](0090-pas-de-bataille-dans-un-fil.md) | accepté |
| 0091 | [Parallélisme de la fin de tour](0091-parallelisme-de-la-fin-de-tour.md) | accepté |
| 0092 | [Sélection native du quadtree de relief (PB3g)](0092-quadtree-de-relief-natif.md) | accepté |
| 0093 | [Rester sur Godot (pas de migration vers Unreal)](0093-rester-sur-godot.md) | accepté |
| 0094 | [Postures d'armée et ouverture d'embuscade (lot CV3-1)](0094-postures-embuscade.md) | accepté |
| 0095 | [Contrôles de bataille façon Total War (lots CB)](0095-controles-bataille-tw.md) | accepté |
| 0096 | [Animation vivante (lots AN1)](0096-animation-vivante.md) | proposé |
| 0097 | [Gabarit d'interface et d'étalonnage (chantier PO)](0097-gabarit-interface-et-etalonnage.md) | accepté |
| 0098 | [Titres féodaux au-dessus des factions (chantier FE)](0098-titres-au-dessus-des-factions.md) | accepté |
| 0099 | [Règles de feu lues depuis `data/` et ordre « Incendier »](0099-regles-feu-depuis-data-et-ordre-incendier.md) | accepté |
| 0100 | [Ordre public pondéré et seuil de révolte (lot RS-B)](0100-ordre-public-pondere-et-revoltes.md) | accepté |
| 0101 | [Sort de la ville prise (TW2-T1)](0101-sort-de-la-ville-prise.md) | accepté |
| 0102 | [Reconstitution des armées et réserves de recrutement (lot TW2-T2)](0102-reconstitution-reserves-recrutement.md) | accepté |
| 0103 | [Compagnies de mercenaires (lot TW2-T3)](0103-compagnies-de-mercenaires.md) | accepté |
| 0104 | [Albédo de détail généré des figurines fines (lot GA1)](0104-albedo-de-detail-des-figurines.md) | accepté |
| 0105 | [Textures du sol de bataille en 2k et macro-variation (lot GA2)](0105-textures-2k-et-macro-variation.md) | accepté |
| 0107 | [Lisibilité et rythme de la destruction en siège (lot SB)](0107-lisibilite-rythme-siege.md) | accepté |
| 0108 | [Points de capture en siège et rééquilibrage des assauts (lot TW2 T4)](0108-points-de-capture-siege.md) | accepté |
| 0109 | [Infobulles en sections, chaîne de bulles à Alt maintenu](0109-infobulles-sections-et-chaine-alt.md) | accepté |
| 0110 | [IA féodale : points d'appel du cœur, ordres féodaux, alliances d'un vassal (chantier FE, lot F5)](0110-ia-feodale.md) | accepté |
| 0111 | [Plafond d'opinion par motif et ordre de démolition (lot RS-C)](0111-plafond-opinion-et-demolition.md) | accepté |
| 0112 | [Traditions d'armée (lot TW2-T5)](0112-traditions-armee.md) | accepté |
| 0113 | [Sonde c7a : plancher du trésor de la France abaissé à 15 000 livres (lot RS-M)](0113-sonde-c7a-plancher-tresor-france.md) | accepté |
| 0114 | [Ost effectif, lien féodal sans alliance, commise de Guyenne (lot FE F8)](0114-ost-effectif.md) | accepté |
| 0115 | [Emprise Oural–Méditerranée](0115-emprise-oural-mediterranee.md) | n/d |
| 0116 | [Terrains, climats, religions et mers de l'Est](0116-terrains-religions-de-l-est.md) | n/d |
| 0117 | [Garde du seigneur : l'unité de la capitale entretenue par le domaine (lot OMR R3)](0117-garde-du-seigneur.md) | accepté |
| 0117 bis | [Lanceur depuis les sources (macOS, Linux, Windows)](0117-lanceur-depuis-les-sources.md) | accepté |
| 0118 | [Copies GPU compressées des rasters monde de la carte de campagne](0118-copies-gpu-compressees-de-la-carte.md) | accepté |
| 0119 | [Portée de planification de l'IA (index partagés en lecture seule)](0119-portee-de-planification-ia.md) | remplacé par l'ADR 0205 |
| 0121 | [Relief fin dans le cadre monde, palier 1 étendu à tout le monde OM](0121-relief-fin-cadre-monde.md) | accepté |
| 0122 | [Une décision expirée applique l'option de l'IA](0122-expiration-par-choix-ia.md) | accepté |
| 0123 | [Budget de pixels de la mise à l'échelle 3D sur écran HiDPI](0123-budget-de-pixels-hidpi.md) | acceptée |
| 0124 | [Deux vues de la carte de campagne](0124-deux-vues-de-campagne.md) | acceptée |
| 0126 | [Types de places assiégées (château, bourg fortifié, cité)](0126-types-de-places-assiegees.md) | accepté |
| 0127 | [Missions de campagne à court terme](0127-missions-de-campagne.md) | accepté |
| 0128 | [Plafond d'unités par armée et engins de siège construits sur place (lot NT5)](0128-plafond-et-engins-de-siege.md) | accepté |
| 0129 | [Fondu entre clips d'un cycle de mêlée et clips propres des rôles (NT7)](0129-fondu-cycle-et-clips-de-role.md) | accepté |
| 0135 | [Ancre de style et redessin guidé du kit d'interface](0135-ancre-de-style-et-redessin-guide-du-kit.md) | acceptée |
| 0136 | [Figurines et bâtiments semi-réalistes (scans CC0, usure procédurale)](0136-figurines-et-batiments-semi-realistes.md) | acceptée |
| 0137 | [Décor de campagne : arbres en cartes et imposteurs, herbe proche, ombres par préréglage](0137-decor-de-campagne-imposteurs-et-herbe.md) | acceptée |
| 0138 | [Villes à l'échelle 1:1 à toutes les hauteurs de la vue 3D](0138-villes-1-1-a-toutes-hauteurs.md) | acceptée |
| 0139 | [Routes maritimes](0139-routes-maritimes.md) | accepté |
| 0140 | [Pipeline image-vers-3D pour le décor de bataille (GA3-L1)](0140-pipeline-image-vers-3d.md) | acceptée |
| 0141 | [Passages de rivière sur la carte de campagne](0141-passages-de-riviere-en-campagne.md) | accepté |
| 0142 | [Sol « satellite » : carte de couleur précalculée](0142-sol-satellite-carte-de-couleur.md) | appliquée |
| 0143 | [Habillage de la carte de campagne par biomes](0143-habillage-par-biomes.md) | appliquée |
| 0144 | [Ville détaillée plus haut, une seule teinte de toits de loin](0144-ville-detaillee-plus-haut.md) | acceptée |
| 0145 | [Voix de combat criées : ElevenLabs v3 et cris de guerre en chœur](0145-voix-criees-elevenlabs.md) | n/d |
| 0146 | [Deux fois plus de régiments par bataille](0146-regiments-doubles.md) | n/d |
| 0146 bis | [Sommation de paix du suzerain au joueur](0146-sommation-de-paix-du-suzerain.md) | n/d |
| 0148 | [Mesurer l'IA avant de la changer : sondage de bataille et duel A/B de campagne](0148-mesure-de-l-ia.md) | n/d |
| 0149 | [fal.ai hors plafond v1 pour le chantier TB (campagne façon Thrones of Britannia)](0149-fal-ai-hors-plafond-campagne-tob.md) | remplacée par l'ADR 0152 |
| 0149 bis | [Le paquet de relief se met à jour tout seul](0149-paquet-de-relief-automatique.md) | n/d |
| 0150 | [Saisons de la carte : écart posé sur la carte de couleur, réglages dans `data/ui/`](0150-saisons-de-carte-donnees-et-surcouches.md) | n/d |
| 0151 | [Un signe par ville ; signes d'évènement réservés à une couche](0151-signes-de-carte-reserves.md) | n/d |
| 0152 | [Chantier TB sans fal.ai](0152-chantier-tb-sans-fal-ai.md) | acceptée |
| 0153 | [Lanceur Windows en `.exe`, commité à la racine](0153-lanceur-windows-exe.md) | accepté |
| 0154 | [Rotation musicale mélangée, persistante, et liste de guerre élargie](0154-rotation-musicale.md) | n/d |
| 0155 | [Le rouge est réservé aux ennemis sur la carte de campagne (lot EN)](0155-rouge-reserve-aux-ennemis.md) | accepté |
| 0156 | [Lumière de la carte : SSIL gardé, SDFGI écarté, brume du matin dans le shader du sol](0156-lumiere-de-carte-ssil-sdfgi-brume.md) | acceptée |
| 0157 | [Traces de guerre : mémoire du rendu, masque de terroir à son échelle](0157-traces-de-guerre-memoire-du-rendu.md) | n/d |
| 0158 | [Carte généralisée : positions vraies, objets grossis d'un facteur constant](0158-carte-generalisee-objets-grossis.md) | acceptée |
| 0159 | [Mises à jour automatiques : le lanceur suit la branche `stable`](0159-mises-a-jour-automatiques.md) | accepté |
| 0160 | [Armées sur la carte : cible survolée, ost hors de la ville, échelle sous-linéaire](0160-armees-selection-survol-echelle.md) | n/d |
| 0161 | [Habillage généralisé : forêts en volume et eaux lisibles à hauteur de jeu](0161-habillage-generalise-forets-eaux.md) | accepté |
| 0162 | [Bâtiments hors les murs et croissance des villes : maquettes assemblées du kit, taille tenue à l'écran](0162-batiments-hors-les-murs-assembles-du-kit.md) | acceptée |
| 0163 | [Côtes et mers par région : polygones dans `data/map/`, matières procédurales](0163-cotes-et-mers-par-region.md) | n/d |
| 0164 | [Assets libres : matière photographiée ou simulée, recodée au format du jeu (FA)](0164-assets-libres-photographies-et-simulations.md) | accepté |
| 0165 | [Faction croisée : une colonie pour base, la Ferveur pour assise](0165-croises-ferveur.md) | n/d |
| 0166 | [Musique de campagne calme : luth, vihuela, luth-clavecin, violes](0166-musique-de-campagne-calme.md) | n/d |
| 0167 | [Voyages maritimes depuis un port](0167-voyages-maritimes-depuis-un-port.md) | accepté |
| 0168 | [Relief de campagne vivant (fin de l'estompage)](0168-relief-vivant.md) | accepté |
| 0169 | [Performance de la carte : densité du relief en pixels rendus, pas de MSAA sur la carte](0169-performance-carte-pixels-rendus.md) | acceptée |
| 0174 | [Formations historiques en données et reformation progressive](0174-formations-historiques-et-reformation.md) | n/d |
| 0175 | [Lisibilité du territoire : possession, occupation, position](0175-lisibilite-du-territoire.md) | n/d |
| 0176 | [Engins de siège contre les régiments](0176-engins-de-siege-contre-les-regiments.md) | n/d |
| 0177 | [Eaux de 1340 dans le masque terre : lacs historiques, retenues modernes](0177-lacs-historiques-et-retenues.md) | n/d |
| 0178 | [Domaine du seigneur, évènements proportionnels, garnison de la capitale hors coût fixe (lot LR-04)](0178-domaine-du-seigneur.md) | accepté |
| 0179 | [Pas de déshérence pour les sièges électifs, branches cadettes des lignées éteintes](0179-lignees-electives-branches-cadettes.md) | n/d |
| 0180 | [Garnison par habitant, prime d'occupation et plafond d'opinion à la lecture (lot LR-07)](0180-garnison-par-habitant-et-occupation.md) | accepté |
| 0181 | [Prévision de bataille alignée sur la résolution automatique](0181-prevision-alignee-sur-resolution.md) | n/d |
| 0182 | [Acceptation diplomatique déterministe pour le joueur](0182-acceptation-diplomatique-deterministe.md) | n/d |
| 0183 | [Économie à l'échelle (lot A6-L3)](0183-economie-a-l-echelle.md) | n/d |
| 0184 | [Durée des batailles : mesure, cadence en données, et limite des leviers de combat](0184-duree-des-batailles.md) | appliqué par le lot A6-L13b** |
| 0185 | [Barre des emplacements de colonie](0185-barre-des-emplacements.md) | n/d |
| 0186 | [La CI suit la version de Godot installée](0186-ci-suit-godot-installe.md) | n/d |
| 0187 | [Sources d'animation par famille](0187-sources-animation.md) | n/d |
| 0188 | [Bêtes animées en shader de sommets, masques par position](0188-betes-animees-shader.md) | n/d |
| 0189 | [Animations tirées de vidéos libres](0189-animations-tirees-de-videos-libres.md) | n/d |
| 0190 | [Génération d'images locale et gratuite (mflux, Z-Image Turbo)](0190-generation-images-locale-mflux.md) | n/d |
| 0191 | [Plancher d'échelle 3D à 0,4 en plein écran HiDPI](0191-plancher-echelle-plein-ecran-hidpi.md) | acceptée |
| 0192 | [Météo de campagne cuite une fois par tour](0192-meteo-cuite-par-tour.md) | acceptée |
| 0200 | [Chantier SC : une seule voie de code](0200-chantier-simplification.md) | n/d |
| 0201 | [Suppression de la bataille navale 3D](0201-suppression-bataille-navale-3d.md) | n/d |
| 0202 | [Une proposition diplomatique est un traité d'articles](0202-traites-articles.md) | accepté |
| 0203 | [La pyramide de relief devient obligatoire](0203-pyramide-relief-obligatoire.md) | accepté |
| 0204 | [Semis de végétation de campagne : Rust seul](0204-vegetation-semis-rust-seul.md) | n/d |
| 0205 | [Le cache de planification est un objet explicite](0205-plancache.md) | accepté |
| 0205 bis | [Retrait du village de bataille B5](0205-retrait-village-bataille.md) | accepté |
| 0206 | [Chargeur Rust des données vectorielles de la carte](0206-chargeur-carte-rust.md) | accepté |
| 0207 | [Un seul moteur de missions, piloté par `data/missions.json`](0207-missions-donnees.md) | accepté |
| 0208 | [Sauvegardes : version 9, plus de compatibilité ascendante](0208-sauvegardes-version-9.md) | accepté |
| 0209 | [Actions d'agents pilotées par une table `ActionSpec`](0209-agents-action-spec.md) | n/d |
| 0210 | [Procédé gratuit des assets 3D : Qwen / Z-Image → TRELLIS (HF) → SF3D](0210-procede-assets-3d-gratuit.md) | accepté |
| 0211 | [Charte semi-réaliste révisée (trois états, figurines générées, palette contrôlée)](0211-charte-semi-realiste.md) | accepté |
| 0212 | [Les modèles 3D générés voyagent en paquet de release, pas par git](0212-paquet-de-modeles-generes.md) | n/d |
| 0213 | [Parcellaire du sol : parcelles de Voronoi, régions dominantes, clairières](0213-parcellaire-voronoi-clairieres.md) | n/d |
| 0214 | [Les lieux de la carte affichent les glb générés, par sous-famille, à toute distance](0214-maquettes-modeles-generes.md) | n/d |
| 0215 | [Eau des lacs façon mer ME1, routes maritimes discrètes](0215-eau-des-lacs-et-routes-maritimes-discretes.md) | n/d |
| 0216 | [Plus de villages, moins de villes sur la carte de campagne](0216-plus-de-villages-moins-de-villes.md) | n/d |
| 0217 | [glb générés sur et au bord de l'eau (lot DN-FLEUVE)](0217-glb-generes-sur-l-eau.md) | n/d |
| 0218 | [Roches générées DN sur la carte de campagne](0218-roches-dn-campagne.md) | accepté |
| 0219 | [Codex du décor naturel et bulle de survol différée](0219-codex-du-decor-naturel.md) | accepté |
| 0220 | [La campagne vivante hors champs : une couche à règles en données, albédo partagé et réduit](0220-campagne-vivante-hors-champs.md) | n/d |
| 0221 | [Forêts de campagne : modèles générés, peuplements par massif, forêt dominante](0221-forets-modeles-generes-et-peuplements.md) | n/d |
| 0222 | [champs, vergers et vignes en modèles 3D générés (DN-CHAMPS)](0222-champs-modeles-generes.md) | n/d |
| 0230 | [Effets secondaires de bataille retirés](0230-effets-secondaires-bataille-retires.md) | accepté |
| 0231 | [Carte : suppression de replis morts](0231-carte-replis-morts.md) | accepté |
