# bm-settings (client Bevy, lot `client/src/settings/`)
État : modèle `Settings` + persistance JSON, écran à onglets (table `rows.rs`), application en direct, dump `settings` : faits, preuve par script (curseur musique → `AudioSettings` + fichier).
Reste : réaffectation des touches (Commandes : `KeyBindings`/`BattleHotkeys` Godot, pas de port Rust), taille du texte (le kit ne la lit pas), qualité graphique / upscale / textures / contraste / daltonisme (champs exposés, rien ne les lit), résolution forcée au démarrage (volontairement non appliquée), bornes des listes (figurines max, etc.).
