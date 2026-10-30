# Security

Report problems in private:

https://github.com/buntatoes/anvil-json/security/advisories/new

Do not put the details in a public issue.

## What is checked

- Input larger than the size cap is rejected. The default is 16 MiB.
- Nesting deeper than the depth cap is rejected. The default is 64.
- Callers that use `parse_json_bounded` can set both caps lower.

Version 0.3.2 is current.
