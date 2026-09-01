# Provenance and licensing notice

`odoo-mcp` is an **independent, clean-room implementation**.

It is built from public interfaces only:

- The **Model Context Protocol** specification
  (https://modelcontextprotocol.io), implemented directly over the stdio
  transport.
- The **Odoo external API** — the documented `/jsonrpc` endpoint with the
  `common.authenticate` and `object.execute_kw` services
  (https://www.odoo.com/documentation, "External API").

Protocols, documented API method signatures, and standard model/method names
are public interfaces, not authored code. This project was written from those
documents.

**It is not derived from, and does not incorporate, source code from any other
Odoo MCP server or from Odoo itself** — in particular none from any GPL-,
LGPL-, AGPL-, or MPL-licensed project. No third-party source was copied or
translated. This clean-room provenance is what allows the project to be offered
under the license in `LICENSE.md`.

Odoo is a trademark of Odoo S.A. This project is independent and not affiliated
with or endorsed by Odoo S.A.

If you believe any part of this code reproduces copyrighted expression from
another project rather than implementing a public interface, please open an
issue; we will investigate and remove it.
