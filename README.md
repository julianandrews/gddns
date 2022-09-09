# gddns

Dynamic DNS client for Google Dynamic DNS.

## Usage

See `gddns --help` for detailed options.

### Config file (recommended)

Create a config file at `/etc/gddns/config.toml` with your hostname and auth info:

```
[hosts]
"<your_hostname>" = { username = "<your_username>", password = "<your_password>" }
```

See `example-config.toml` for a concrete example.

Create a directory at `/var/cache/gddns` and ensure you have write access to it.

Invoke with:

```
gddns
```

### Direct invocation

You can also invoke `gddns` directly without any setup:

```
gddns update-host <your_hostname> --username <your_username> \
    --password <your_password> --cache-dir /tmp/gddns
```

Note that repeated requests to update your IP when it hasn't changed can result
in blacklisting so you should be careful not to invoke `gddns` without a
persistent cache directory to prevent unecessary requests.
