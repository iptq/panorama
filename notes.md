design ideas
---

- instead of dumb search, have like an omnibar with recency info built in?
  - this requires some kind of cache and text search
- mail view has like a "filter stack"
  - initially, this is empty, but when u do `/` u can add stuff like `acct:personal`, or `date<2020-03` or `has:attachment` or `from:*@gmail.com`
  - then, when u hit enter, it gets added to a stack and u can like pop off filters
  - example wld be liek `[acct:personal] [is:unread] [subject:"(?i)*github*"]` and then when u pop off the filter u just get `[acct:personal] [is:unread]`
- tmux-like windows
  - maybe some of the familiar commands? `<C-b %>` for split for ex,
- gluon for scripting language
  - hook into some global keybinds/hooks struct
  - need commands:
    - create dir
    - move email to dir
- transparent self-updates?? this could work with some kind of deprecation scheme for the config files
  - for ex: v1 has `{ x: Int }`, v2 has `{ [deprecated] x: Int, x2: Float }` and v3 has `{ x2: Float }`  
    this means v1 -> v2 upgrade can be done automatically but because there are _any_ pending deprecated values being used
    it's not allowed to automatically upgrade to v3

imap routine
---

- basic tcp connection is opened
- if tls is "on", then immediately perform tls handshake with the server
- if tls is "starttls", check starttls capability
  - if the server doesn't have starttls capability, die and report to the user
  - if the server _does_ have starttls, exit the read loop and perform tls handshake over current connection
- at this point, tls should be figured out, so moving on to auth
- check if the auth type that the user specified is in the list of auth types (prob support plain and oauth2?)


list of shit to do
---

- [x] starttls impl
- [ ] auth impl
  - [ ] auth plain impl
- [ ] fetch impl
