# Lot stab1 — stabilité du client Bevy (terminé)

Branche `bm/stab1` (base 60b30bc6e). Essais : `run_bg.sh` muet hors écran, boucle de 32 batailles
(Crécy historique, siège de Paris, `state Battle`, bataille personnalisée au crépuscule ;
`CENT_ANS_BATTLE_SPEED=8`, `CENT_ANS_BACKTRACE=1`), code de fin relevé par le marqueur
`AGENT_EXIT`, rapports `~/Library/Logs/DiagnosticReports`, `sample` du processus au-delà de 200 s.

1. **Arrêt silencieux ~3 s après le début d'une bataille** : non reproduit (32 batailles, 32 sorties
   par le script, aucune panique, aucun rapport de plantage le 16/10). Piste testée et écartée :
   binaire réécrit par un second `run_bg.sh` pendant l'exécution (l'app survit). Reste à
   surveiller ; les rapports du 15/10 sont des lancements cassés par une recompilation en cours
   (dylib manquante, signature invalide au chargement), pas des arrêts en bataille.
2. **Blocage à la sortie** : non reproduit (32 sorties). Cause relevée par perf3 et confirmée dans
   le code de Bevy 0.20 (`bevy_winit::system` garde ses `WindowWrapper` une image, mais le monde de
   rendu garde la fenêtre par `RawHandleWrapper` et peut lâcher la dernière référence sur le fil de
   rendu → `Window::drop` macOS attend le fil principal, bloqué dans `RenderAppChannels::drop`).
   Correctif préventif : `app/window_keepalive.rs` (copie du `RawHandleWrapper` jamais libérée).
3. **« Mesh index 17 contains no vertices »** : sonde → `GoreDrops` (`battle/decor/gore.rs`) créé
   avec un maillage vide. Corrigé : l'entité naît sans maillage et masquée, le maillage n'est
   créé qu'au premier jet de gouttes, masqué quand il n'y en a plus. 0 avertissement sur 16 batailles.
4. **Lueur des feux de camp** : `battle/effects/campfires.rs` alimente `SmokeSources`. Bataille
   personnalisée été, temps clair, crépuscule : 6 sources (3 par camp en qualité haute), lueur
   `lit` 4, obscurité 0,10. Crécy (pluie) et siège de Paris (siège, sans camp) : 0, comme Godot.
5. **`params.fx.x` partagé** : pas de bogue visible (les morts du mode 3 ont `volley_time` -1000),
   mais conflit de sens avec Godot qui a deux uniformes (`cycle_blend`, `role_blend`). Paramètre
   propre `hitn.y` (`set_role_blend`) pour les servants ; test.
6. **Clics `cb:season` / `cb:weather` « perdus »** : la composition de départ est relue de
   `custom_battle_last.json` (dossier du joueur), réécrite à chaque lancement de script ; un clic
   relatif atterrit donc sur une saison différente d'une exécution à l'autre (le fichier du joueur
   avait été réécrit par un agent le 16/10 à 19:09, pluie). Correctif : dossier propre au
   processus pour les exécutions d'agent sans `CENT_ANS_PREFS`. Preuve : 4 × `cb:season:1` =
   été → automne → hiver → printemps → été ; météo 1, 1, -1 = clair.

## Restes
- Points 1 et 2 non reproduits : si un arrêt revient, lancer `RUST_BACKTRACE=1` et `sample <pid>`
  (script de boucle : relever `AGENT_EXIT`, rapports `.ips`).
- `battle/units/smoke.rs` dit encore « Not ported: the night glow » (porté dans `effects/glow.rs`).
- Godot tire le décalage des feux derrière les lignes au hasard (graine) ; ici hachage déterministe.
