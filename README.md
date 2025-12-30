# `asker`

`asker` allows daemons to asker for user input.

<./nix/nixos-module.nix> provides options for the `asker` system.
An `asker` configuration has several *keys*, which are different inputs that can be requested by name.
A key can be requested by calling `asker KEY`.

<./nix/home-manager-module.nix> provides options for `asker-prompt`.
`asker-prompt` runs as a user service so that it can create windows.
When a key is requested, `asker-prompt` creates a graphical dialog that describes the request and asks for input.

For each key there is a corresponding group named `asker-key-{KEY}`.
`asker KEY` creates a request in a way that only members of the `asker-key-{KEY}` group can read the response.
Read more about the implementation in [the design notes](./notes/2025-12-26-design.md).

## Motivation

I run [Syncthing](https://syncthing.net/) to keep my [KeepassXC](https://keepassxc.org/) in sync across devices.
At the same time I have [`syncthing-merge`](https://github.com/LightAndLight/syncthing-merge) running to handle any conflicts due to concurrent database edits.
It calls `keepassxc-cli merge` to merge conflicting databases, which requires my database password.
`syncthing-merge` runs as a system service under its own user,
so I wasn't able to use a graphical dialog program like [`zenity`](https://en.wikipedia.org/wiki/Zenity) to ask for the password.
My solution is to run a user service that can create graphical dialogs in response to requests from system daemons,
while enforcing minimal access to the data that the user enters.

## Prior art

I looked into using existing keyring programs via the [D-Bus Secret Service API](https://specifications.freedesktop.org/secret-service/latest/ref-dbus-api.html),
but I couldn't figure out how to control access to individual secrets.
I know exactly which services should be allowed to access particular secrets, and I want to enforce that.
In particular, I don't want my logged-in user to have universal access to these secrets, because then any program I run can read them (see also: [recent discussion of this issue on Hacker News](https://news.ycombinator.com/item?id=46278857)).
I also found that these keyring programs aren't suited for ephemeral data; they store secrets for a while.

It might be possible to achieve this using pure D-Bus with [access control policies](https://dbus.freedesktop.org/doc/dbus-daemon.1.html#:~:text=The%20%3Cpolicy%3E%20element).
I haven't looked into this because after I decided that the Secret Service API was insufficient,
I figured that rolling my own protocol would be easier than learning D-Bus.
