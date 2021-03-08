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
    - SELECT: incomplete args
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
    - SEARCH: incomplete args
    - FETCH: incomplete args
    - STORE: not yet implemented
    - COPY: not yet implemented
    - UID: incomplete args
- RFC2177 (IMAP4 IDLE)
  - IDLE: works?
