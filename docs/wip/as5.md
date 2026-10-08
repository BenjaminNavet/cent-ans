# AS5 — restes statiques en shader (feat/as5)

État : (a) flammes de carte, (b) onde de vent des bannières de maquette, (c) balancement des imposteurs d'arbres de bataille : faits. Réglages dans `data/fx/map_fire_wind.json`, A/B `--no-as5`. Test `game/tests/as5_test.gd`, planche `game/tests/as5_shot.gd` (écrit user://as5_shot.png).

Restes : bivouacs de campagne (aucun foyer de bivouac n'existe côté carte ; ajouter une liste de points dans `LifeEffects` quand l'état d'armée à l'arrêt le fournira, lot AS2 / army_*.gd) ; ombre des imposteurs (`battle_impostor_shadow.gdshader`) non balancée ; jugement visuel en jeu (AS7).
