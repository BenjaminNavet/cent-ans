# AS — inventaire des animations (08/10)

Aucun AnimationPlayer ni Skeleton3D : tout passe par (1) textures d'os lues en shader sur
MultiMesh, (2) shaders de sommets/particules (`TIME`), (3) GDScript (Tween, pièces nommées,
Jolt pour l'image seulement).

| Famille | Méthode actuelle | Manques principaux |
|---|---|---|
| Humains (rig `human`, 70 clips) | Keyframé Blender sur actions Quaternius CC0, cuit en texture d'os (ADR 0014, 0089) | pas de pas de côté, recul, rotation sur place ; kit `--coarse-figures` en retard |
| Mêlée | mélange : NT14 vidéo (guard, overhead, thrust), FA3 Mesh2Motion (parry), keyframé (slash, hit) | lame décalée sur l'estoc, slash/hit/chute à filmer ; qualité hétérogène |
| Morts, blessés, gore | clips death/wounded/crawl/flee, balistique en shader | renversés qui glissent, armes à plat sur pente |
| Duels | clips existants synchronisés (BV3) | aucun clip apparié, pas de mort en duel |
| Mouvement secondaire | procédural shader (AN1a) | vitesse par régiment, continue en pause |
| Chevaux (23 clips) | keyframé (FG4) | pas de trot, de cheval seul, de virage ; chevaux du camp statiques |
| Étendards, musiciens | clips de rôle + shader d'onde | `maquette_banner` statique ; hampe de carte rigide |
| Engins de siège | GDScript procédural sur pièces nommées (ADR 0128, 0176), servants skinnés | servants immobiles, ne portent rien ; bélier hors avancée |
| Murailles, portes | Jolt (image) | blocs grossiers |
| Navires | houle 4 sinus, voiles/avirons shader | pas de sillage ni d'équipage en campagne |
| Armées sur la carte | figurines V2 idle/walk | pieds qui glissent (cadence fixe) |
| Folk, scènes de vie (FK) | villageois skinnés + trajet shader ; bêtes/charrettes rigides | **animaux sans animation**, trajets rectilignes en boucle |
| Villes, moulins | statiques + ailes/panaches shader | pas de fenêtres éclairées |
| Végétation | shaders de vent | imposteurs d'arbres de bataille statiques |
| Eau | shaders | rien de signalé |
| Feu et fumée | flipbooks FA2 (Unity Labs CC0) | aucune flamme sur la carte |
| Météo | particules + nuées | pas de météo en mer |
| Oiseaux | shader MultiMesh | — |
| Imposteurs soldats | atlas 8 angles × 4 états × 4 images | engins sans imposteurs |
| Caméras | GDScript lissé, cinématique | — |
| UI | Tweens, portraits fixes vieillissants | aucun portrait animé, sceaux statiques |

Constats : un seul pipeline personnages (rig Quaternius → texture d'os) ; aucune animation ne
vient de fal/Meshy ; jugement en jeu = point ouvert le plus fréquent (NT7, NT10, NT14, FA3, FK,
AN1a validés sur planches) ; chaque lot garde un drapeau d'A/B.
