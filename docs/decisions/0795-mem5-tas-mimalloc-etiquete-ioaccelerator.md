# 0795 — MEM5 : les « 3 Go graphiques non comptés » sont le tas mimalloc

Date : 2026-10-29. Statut : acceptée. Amende l'ADR 0794 (point 3 du contexte) et `docs/performance-memoire.md`.
Lot MEM5. Détails : `docs/wip/lot-mem5.md`.

## Contexte
L'ADR 0794 laissait ~3 Go d'empreinte de campagne « graphiques, possédés mais non comptés par Metal », insensibles aux
images, aux pipelines, au terrain, à la végétation et à la qualité. Budget visé : 4 Go ; mesuré : 5,4-5,8 Go au repos.

## Constat (mesures, campagne au repos, release, Ultra, hors écran, relief chargé)
1. **mimalloc étiquette ses pages avec le tag VM 100** (option `os_tag`, défaut 100), qui vaut
   `VM_MEMORY_IOACCELERATOR` (`mach/vm_statistics.h`). `footprint` et `vmmap` rangent donc tout le tas Rust sous
   `IOAccelerator` (sans « (graphics) ») : 51 régions de 128 Mio rw- (arènes mimalloc), 2 660-2 716 Mo.
   Trois preuves : (a) `--features system-alloc` : `IOAccelerator` tombe à 1,2 Mo, `Malloc Large/Small` prend le
   relais ; (b) `MIMALLOC_OS_TAG=250` : `IOAccelerator` 1 Mo, `App-Specific Tag 11` 2 734 Mo ; (c) la taille des
   régions est celle des arènes mimalloc.
2. Décomposition de l'empreinte (5 790 Mo, t+10 s) : GPU non mappé 2 732 (≈ `currentAllocatedSize` 2 492 : images
   1 995 du `gpucensus` + ≈ 500 de maillages et cibles), **tas CPU 2 664**, GPU mappé 168, reste ≈ 230. Il n'y a pas
   de mémoire graphique inexpliquée ; il y a un tas CPU de 2,7 Go.
3. Contenu du tas (`MallocStackLogging=lite` + `malloc_history -allBySize`, binaire `system-alloc`, 3 078 Mio vivants) :
   végétation ≈ 920 (2 atlas d'imposteurs 165 Mo gardés en copie CPU après la chaîne de mips ; rasters 7080x6104 du
   semis ≈ 590 : hauteurs en octets 86, rivière x2, forêt, cultures, biomes, terre, `wetlands` RGB 130...) ; relief E0
   124 ; ids de provinces côté client 82 et raster provinces/navgrid du cœur 82 ; atlas de polices 89 ; copies CPU des
   glTF 89 ; ECS 79 ; masque terre décodé deux fois 48 + 41 ; countryside 92 ; coast rocks 41 ; LifeData 43 ; warp 42 ;
   rubans d'eau 64 ; pilote Metal (ressources poolées) 164.

## Décision
1. Atlas d'imposteurs de la végétation : passés en `RENDER_WORLD` à l'installation de la chaîne de mips (l'envoi GPU
   prend les pixels ; plus aucun lecteur CPU). L'albédo garde sa copie tant que le travail des normales n'a pas copié
   sa couverture.
2. `wetlands.png` du semis : un seul plan max(R, G) (seule lecture du masque), au lieu de RGB.
3. `land_mask.png` : décodé une fois (`armies::landmask::shared_land_mask`, cache `Weak`) pour les ordres et l'eau douce.
4. `mem_protocol.sh` : lance le client avec `MIMALLOC_OS_TAG=250` et affiche séparément GPU non mappé, GPU mappé et
   tas. Toute lecture de `footprint` d'un binaire mimalloc doit lire `IOAccelerator` (non graphics) comme du tas CPU.

## Conséquences
- Campagne au repos : tas 2 712 -> 2 337-2 389 Mo, empreinte 5 684-5 842 -> 5 382-5 486 Mo (2 runs chacun, même
  machine, charge 19-26). Fin de banc carte (1 run chacun, charge 60-83, bruité) : empreinte 7 492 -> 7 426 Mo, pic RSS 7 137 -> 6 956 Mo ; seuils de `perf_thresholds.json` inchangés (gain dans le bruit des descentes, machine non calme). Rendu inchangé : `gpucensus` identique, une capture `camp_0600` (impostors lointains normaux).
- Le budget de 4 Go reste hors d'atteinte sans toucher à la qualité : GPU ≈ 2,7 Go (textures 2 Go) + tas ≈ 2,3 Go.
  Pistes suivantes, par gain : doublons de rasters (provinces client/cœur 82, `river_bed` x3, hauteurs du semis 86 via
  `MapRasters.height` du cœur), rasters du semis à partager avec le terrain, copies CPU des glTF, compression BC des
  atlas d'imposteurs (330 Mo GPU en RGBA8).
- Le joueur voit toujours le tas sous « IOAccelerator » dans les outils système : `os_tag` se règle par variable
  d'environnement avant le premier `malloc`, pas depuis `main`.
