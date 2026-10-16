# VQ cground — sol de campagne de près

## État (17/10)
- Vérification : le port Bevy contient déjà parcelles Voronoï (`fp_parcels`), champs peints (`fields_apply`, pack `tx_campaign_parcels`), terroirs, saisons par culture, variation macro GA4, grain micro. L'écart « aplats » de `bm-fidelity.md` vient donc du réglage / du rendu, pas d'une absence de code : diagnostic en cours sur capture de base.
- Plan : fichier `ground_detail.wesl` (importé par `terrain.wesl`, appel minimal dans le chemin d'albédo), bouton par préréglage dans `data/fx/render_quality.json`.

## Reste
Tout (build release en attente, machine très chargée).
