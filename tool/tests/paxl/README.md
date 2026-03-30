# PAX-L Conformance Test Suite

Language-agnostic test fixtures for PAX-L parser implementations.

## Structure

- `roundtrip/` — PAX files + expected PAX-L output. Any parser must:
  - Parse the .pax file
  - Serialize to PAX-L
  - Deserialize the PAX-L back to a PaxFile
  - Verify all tiles, palettes, and metadata survive the roundtrip

- `invalid/` — Malformed PAX-L files that strict-mode parsers must reject:
  - `missing_rows.paxl` — declares [4] rows but only provides 2
  - `unknown_directive.paxl` — contains `@unknown_thing` (strict rejects)
  - `delta_chain.paxl` — delta tile references another delta (forbidden)
  - `row_ref_forward.paxl` — `=4` on row 2 (forward reference)
