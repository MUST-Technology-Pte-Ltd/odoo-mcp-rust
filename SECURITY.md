# Security Policy

## Reporting a vulnerability

Email **security@must.com.sg** with details and, if possible, a reproduction.
We aim to acknowledge within 3 business days.

Please report privately first — do not open a public issue for a suspected
vulnerability until we have had a chance to respond.

## Scope notes

`odoo-mcp` holds Odoo credentials from its environment and can only perform
what the authenticated Odoo user is permitted to do; it grants no privilege of
its own. Write operations are disabled unless the operator sets
`ODOO_ALLOW_WRITE=1`. Reports about credential handling, the write gate, or
request construction are especially welcome.

Maintained by MUST Technology Pte Ltd, Singapore — https://must.com.sg
