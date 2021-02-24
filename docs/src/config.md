# Config

Configuration is done by editing `$XDG_CONFIG_HOME/panorama/panorama.toml`. It
follows the [TOML][1] file format, and the data structures are defined in code
at `src/config.rs`.

Example configuration:

```toml
version = "0.1"

[[mail]]
imap.server = "mail.example.com"
imap.port = 143
imap.tls = "starttls"
imap.auth = "plain"
imap.username = "foo"
imap.password = "bar"
```

As one of the primary goals of panorama, the application should automatically
detect changes made to this file after it has started, and automatically
re-establish the connections required. As a result, there's no UI for editing
the configuration within the application itself.

[1]: https://toml.io/en/
