# gddns

Dynamic DNS client.

## Supported services

gddns has been tested on Google domains and No-IP, but should work with any
standard dynamic DNS service.

## Usage

You will need an account with a dynamic DNS provider, and the corresponding
update URL:

- Google Domains: https://domains.google.com/nic/update
- No-IP: https://dynupdate.no-ip.com/nic/update

See `gddns --help` for detailed options.

### Config file (recommended)

Create a config file:

```
# /etc/gddns/config.toml
[hosts]

  [hosts."<your_hostname>"]
  username = "<your_username>"
  password = "<your_password>"
  dyndns_url = "https://domains.google.com/nic/update"
```

See `example-config.toml` for a concrete example.

You will need write access to a cache directory. By default, gddns will try to
write to `/var/cache/gddns`.

Invoke with:

```
gddns
```

### Direct invocation

You can also invoke gddns directly:

```
gddns update-host <your_hostname> --username <your_username> --password <your_password> \
    --dyndns_url "https://domains.google.com/nic/update" --cache-dir /tmp/gddns
```

## Contributing

Pull requests are welcome. For non-trivial changes, please open an issue to
discuss the change before starting.
