# Image des captures sans fenêtre sur le Mac (chantier CF) : Godot Linux arm64 sous
# écran virtuel (Xvfb) avec rendu logiciel Mesa (lavapipe pour Vulkan/Forward+, llvmpipe pour
# OpenGL), plus la chaîne Rust pour compiler la GDExtension Linux. Construite par
# tools/godot_shot.sh ; la version de Godot suit celle de l'hôte (argument GODOT).
FROM rust:1-slim-trixie
RUN apt-get update && apt-get install -y --no-install-recommends \
    xvfb xauth mesa-vulkan-drivers libgl1-mesa-dri libgl1 libegl1 libvulkan1 \
    libxcursor1 libxinerama1 libxrandr2 libxi6 libxkbcommon0 libfontconfig1 \
    libasound2 libpulse0 libdbus-1-3 ca-certificates curl unzip rsync \
    && rm -rf /var/lib/apt/lists/*
ARG GODOT=4.7.2
RUN curl -fsSL -o /tmp/godot.zip \
    "https://github.com/godotengine/godot/releases/download/${GODOT}-stable/Godot_v${GODOT}-stable_linux.arm64.zip" \
    && unzip -q /tmp/godot.zip -d /opt \
    && mv "/opt/Godot_v${GODOT}-stable_linux.arm64" /usr/local/bin/godot && rm /tmp/godot.zip
