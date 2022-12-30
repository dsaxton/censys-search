```
Censys Search API utility

Usage: censys-search [OPTIONS] <COMMAND>

Commands:
  query   Search based on custom query
  ip      Search based on IP address
  dns     Search based on DNS name
  asn     Search based on autonomous system number
  cert    Search based on TLS certificate
  fields  Show all available Censys Search query language fields
  help    Print this message or the help of the given subcommand(s)

Options:
  -i, --api_id <ID>      API ID (if not specified CENSYS_API_ID must be set)
  -s, --secret <SECRET>  API secret (if not specified CENSYS_SECRET must be set)
  -o, --output <FILE>    Output file name
  -n, --no_paging        Disable paging of results
  -h, --help             Print help information
  -V, --version          Print version information
```
