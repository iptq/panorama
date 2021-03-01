IMAP
===

here's the list of RFCs planning to be supported and the status of the
implementation of their commands:

- RFC3501 (IMAP4rev1)
  - any state:
    - CAPABILITY: works
    - NOOP: not yet implemented
    - LOGOUT: not yet implemented
  - not authenticated state:
    - STARTTLS: works
    - AUTHENTICATE: not yet implemented
    - LOGIN: plain only
  - authenticated state:
    - SELECT: not yet implemented
    - EXAMINE: not yet implemented
    - CREATE: not yet implemented
    - DELETE: not yet implemented
    - RENAME: not yet implemented
    - SUBSCRIBE: not yet implemented
    - UNSUBSCRIBE: not yet implemented
    - LIST: not yet implemented
    - LSUB: not yet implemented
    - STATUS: not yet implemented
    - APPEND: not yet implemented
  - selected state:
    - CHECK: not yet implemented
    - CLOSE: not yet implemented
    - EXPUNGE: not yet implemented
    - SEARCH: not yet implemented
    - FETCH: not yet implemented
    - STORE: not yet implemented
    - COPY: not yet implemented
    - UID: not yet implemented
- RFC2177 (IMAP4 IDLE)
  - IDLE: works?
