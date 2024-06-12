> ### :warning: WARNING
> This project is currently under heavy development and is not reccomended
> for any sensitive context!

# orjail.rs

[**orjail**](https://github.com/orjail/orjail) is a tool that lets you
create a jail around a program to force its traffic through the [Tor](https://www.torproject.org/).

This project aims to be a full rewrite that relies on unpriviliged
containers that are natively created using the standard library, while its
predecessor relied on [firejail](https://firejail.wordpress.com/) that by
design relies on privileged namespaces.

It's written in Rust because one of its core features will be to rely on
[arti](https://arti.torproject.org/) to forward the jail traffic through
TOR. Unfortunately this is not convenient at the moment, as this we expect
to run TOR in transparent proxy mode, which [arti does not offer at the
moment](https://gitlab.torproject.org/tpo/core/arti/-/issues/72).

## Requirements

    - slirp4netns
    - tor
    - bubblewrap (Optional)
    - cargo (for building)

For example on Debian Bookworm you should be fine by just installing `tor` and `slirp4netns`:
`sudo apt install tor slirp4netns`

## Installation

At the moment supported by building only:

```
cargo build
```

then you can launch it as `cargo run <command>`, the binary should be saved
in `target/debug/orjailrs`, in case you want to launch it manually.

## Options

> ```bash
> orjailrs [options] [command]
> ```
> **-d --debug**
> Set log level to debug
>
> **-b --bubblewrap**
> Spawn using bubblewrap
>
> **-u --uid**
> User id to spawn inside the container
>
> **-t --tor**
> Tor executable path
>
> **-s --slirp4netns**
> slirp4netns executable path
>
> **-n --namespace**
> Set the name of the network namespace

---
Made with  :heart: by [_to hacklab](https://autistici.org/underscore)
