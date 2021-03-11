panorama
========

[![](https://tokei.rs/b1/github/iptq/panorama?category=code)](https://github.com/XAMPPRocky/tokei)

Panorama is a terminal Personal Information Manager (PIM).

Status: **not done yet**

Read documentation at [pim.mzhang.io][1]

Join chat on Matrix at [#panorama:mozilla.org][3]

Goals:

- **Never have to actually close the application.** All errors should be
  handled gracefully in a way that can be recovered or restarted without
  needing to close the entire application.
- **Handles email, calendar, and address books using open standards.** IMAP for
  email retrieval, SMTP for email sending, CalDAV for calendars, and CardDAV
  for address books. Work should be saved locally prior to uploading to make
  sure nothing is ever lost as a result of network failure.
- **Hot-reload on-disk config.** Configuration should be able to be reloaded so
  that the user can keep the application open. Errors in config should be
  reported to the user while the application is still running off the old
  version.
- **Scriptable.** Built-in scripting language should allow for customization of
  common functionality, including keybinds and colors.

Stretch goals:
- Full-text email/message search
- Unified "feed" that any app can submit to.
- Submit notifications to gotify-shaped notification servers.
- JMAP implementation.
- RSS aggregator.
- IRC client??

Credits
-------

IMAP library modified from [djc/tokio-imap][2], MIT licensed.

License: GPLv3 or later

[1]: https://pim.mzhang.io
[2]: https://github.com/djc/tokio-imap
[3]: https://matrix.to/#/!NSaHPfsflbEkjCZViX:mozilla.org?via=mozilla.org
