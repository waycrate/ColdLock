# ColdLock

<b>ColdLock</b> is a wayland session lock screen built using `iced_sessionlock`. 

## View

![img.png](/images/img1.jpeg)

![img_1.png](/images/img2.jpeg)

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

## Contributing

Contributions are welcome! Feel free to open issues or submit pull requests to help improve ColdLock.


