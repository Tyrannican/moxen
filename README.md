# Moxen - World of Warcraft Addon Manager

I hate Curseforge's client so I'll do it myself.

Not really intended for use by anyone else but it's here if people are interested.

## Prerequisites

You'll need an API Key from [CurseForge](https://docs.curseforge.com/rest-api/#getting-started)

You can use either `CurseForge for Studios` or you can apply for a personal one.
I'd suggest using the `CurseForge for Studios` as I'm unsure if they'll give you a key if, like me, your reason is that you don't like their client.

## Usage

* This project only works for World of Warcraft addons.
* You have to run `moxen init` before using it (only required once).
* All operations that interact with Addons (e.g. tracking and uninstalling) are done using their `Project ID` on CurseForge.

```bash
CLI for installing World of Warcraft addons via CurseForge

Usage: moxen <COMMAND>

Commands:
  init         Initialise Moxen
  track        Track new addons in the registry
  switch       Switch registry to use (retail, ptr, beta, classic, classic-era)
  list         List tracked addons in the registry
  clear-cache  Clear the Moxen file cache
  update       Download the latest version of the addon(s)
  install      Install the addons in the WoW directory
  uninstall    Uninstall the selected Addons
  help         Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```
