# ColdLock

<b>ColdLock</b> is a Wayland session lock screen built using `iced_exwlshell`.

## View

![img1.png](/assets/examples/img1.png)

![img_1.png](/assets/examples/img2.png)

## Installation

### NixOS

For now flake users only.

This repo contains a NixOS Module for coldlock.
To enable module add an input first and import to modules:
```nix
{
  inputs = {
    coldlock.url = "github:waycrate/coldlock";
  }

  outputs = {nixpkgs, coldlock, ...} @ inputs: {
    nixosConfigurations.HOSTNAME = nixpkgs.lib.nixosSystem {
      specialArgs = { inherit inputs; };
      modules = [
        ./configuration.nix
        coldlock.nixosModules.default
      ];
    };
  } 
}
```
After importing you should be able to use it in your configuration.nix file:
```nix
{}: {
    programs.coldlock.enable = true;
}
```

## Other distros

Build and install ColdLock:

```sh
git clone https://github.com/waycrate/ColdLock.git
cd ColdLock
cargo build --release
sudo install -Dm755 target/release/coldlock /usr/local/bin/coldlock
sudo install -Dm644 pam/coldlock /etc/pam.d/coldlock
```

The bundled PAM config uses your distro's `/etc/pam.d/login` service.
Run `coldlock` from your Wayland session to lock the screen.

## Contributing

Contributions are welcome! Feel free to open issues or submit pull requests to help improve ColdLock.
