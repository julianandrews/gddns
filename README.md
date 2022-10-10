# gddns

Dynamic DNS client.

gddns is designed to be a clean and simple dynamic DNS client.

## Supported services

gddns has been tested on Google domains and No-IP, but should work with any
of several other dynamic DNS service using the same DynDNS API.

## Installation

Look for pre-built binaries in the
[releases](https://github.com/julianandrews/gddns/releases).

You can also build from source:

    git clone https://github.com/julianandrews/gddns
    cd gddns
    cargo build --release

and use the `gddns` binary at `target/release/gddns`.

### Debian Based distributions

Install the `.deb` package from the releases page, edit
`/etc/gddns/config.toml` with configuration for your particular domain(s), and
then run:

    sudo systemctl enable --now gddns.service

### Other Linux or MacOS

You will need to:

- create a config file,
- create a cache directory with write access, and
- either
    - run gddns in daemon mode, or
    - periodically run gddns (potentially with a systemd timer or cronjob).

#### Config file

By default gddns looks for a config file at `/etc/gddns/config.toml` and will
update all configured hosts. See `pkg/config.toml` for a concrete example.

#### Cache directory

The user running gddns must have write permissions to the cache directory. By
default this is `/var/cache/gddns`. This can be overridden in `config.toml`, or
by command line option.

Files in the cache directory can be deleted if the cache gets out of sync, in
which case, gddns will send an update request the next time it is run.

## Usage

If properly configured, you can simply run

    gddns

and gddns will update all configured dynamic DNS hosts.

    gddns daemon

will launch gddns and regularly poll your IP address looking for changes.

See `gddns --help` for detailed options.

### Direct invocation

You can also invoke gddns directly for a single host:

    gddns update-host <your_hostname> --cache-dir /tmp/gddns \
        --username <your_username> --password <your_password> \
        --dyndns-url "https://domains.google.com/nic/update"

## Contributing

Pull requests are welcome. For non-trivial changes, please open an issue to
discuss the change before starting.
