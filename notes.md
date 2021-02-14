imap routine
---

- basic tcp connection is opened
- if tls is "on", then immediately perform tls handshake with the server
- if tls is "starttls", check starttls capability
  - if the server doesn't have starttls capability, die and report to the user
  - if the server _does_ have starttls, exit the read loop and perform tls handshake over current connection
- at this point, tls should be figured out, so moving on to auth
- check if the auth type that the user specified is in the list of auth types (prob support plain and oauth2?)

