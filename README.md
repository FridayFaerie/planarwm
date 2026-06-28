<!--
SPDX-FileCopyrightText: © 2026 FridayFaerie
SPDX-License-Identifier: 0BSD
-->

# PlanarWM

2D scrolling window manager for the [River](https://codeberg.org/river/river) compositor, based on [tinyrwm](https://codeberg.org/river/tinyrwm).

## Installation

### Building
```sh
cargo build --release
```

### Set up
```sh
# Place the executable somewhere in PATH
cp ./target/release/planarwm ~/.local/bin/

# Copy the example config to the appropriate location
cp ./example/planarwm.conf ~/.config/river/

# Set up planarwm to be visible in display managers
sudo cp ./example/planarwm.desktop /usr/local/share/wayland-sessions/
```

## Running
```sh
planarwm
```

