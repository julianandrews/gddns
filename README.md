# gddns

Dynamic DNS client.

gddns is designed to be a clean and simple dynamic DNS client which should
work for most people in most situations.

It is designed for periodic execution, and is intended to be used either for
ad-hoc updates or with systemd timers or crontab entries to keep a system up to
date. gddns maintains a cache of recent responses and will only hit the DDNS
server if your public IP address has changed.

## Supported services

gddns has been tested on Google domains and No-IP, but should work with any
dynamic DNS service using the standard dynamic DNS API.

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

    sudo systemctl enable --now gddns.timer

gddns will run every 5 minutes. If you wish to run gddns at a different
frequency, you can create your own systemd timer using `gddns.timer` as a
model or write a simple cronjob.

### Other Linux or MacOS

You will need to:

- create a config file,
- create a cache diretory with write access, and
- periodically run gddns (potentially with a systemd timer or cronjob).

#### Config file

By default gddns looks for a config file at `/etc/gddns/config.toml`. See
`pkg/config.toml` for a concrete example.

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

See `gddns --help` for detailed options.

### Direct invocation

You can also invoke gddns directly:

    gddns update-host <your_hostname> --cache-dir /tmp/gddns \
        --username <your_username> --password <your_password> \
        --dyndns_url "https://domains.google.com/nic/update"

## Contributing

Pull requests are welcome. For non-trivial changes, please open an issue to
discuss the change before starting.
