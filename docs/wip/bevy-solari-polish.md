# solari-polish (23/10)

État : le sol de substitution est abaissé (`ground_sink_m`, 0.4 m, sous le minimum des hauteurs voisines) pour supprimer
l'auto-ombrage ; éclairage du ciel : la caméra de bataille porte déjà un `GeneratedEnvironmentMapLight` (lu par Solari),
mais `AmbientLight` (30 % de l'énergie) est ignoré par Solari -> à fusionner dans l'intensité de l'env map si Solari actif.
Consigne coordinateur : aucune compilation avant ~00:40 (bancs release d'une autre session).
Prochaine étape : compiler, capturer (désactivé / activé), mesurer la part de pixels noirs (moitié basse).
