use clap::{arg, ArgAction, Command};
use reqwest::blocking::Client;
use reqwest::header::{ACCEPT, AUTHORIZATION};
use serde_json::Value;
use std::{env, fs, io::Write, path::Path, process};

mod constants;

fn main() {
    let arg_matches = Command::new("censys-search")
        .version("1.0")
        .about("Censys Search API utility")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .arg(
            arg!(-i --api_id <ID> "API ID (if not specified CENSYS_API_ID must be set)")
                .required(false),
        )
        .arg(
            arg!(-s --secret <SECRET> "API secret (if not specified CENSYS_SECRET must be set)")
                .required(false),
        )
        .arg(arg!(-o --output <FILE> "Output file name").required(false))
        .arg(
            arg!(-n --no_paging "Disable paging of results")
                .required(false)
                .action(ArgAction::SetTrue),
        )
        .subcommand(
            Command::new("query")
                .about("Search based on custom query")
                .arg_required_else_help(true)
                .arg(arg!([query] "Query using the Censys Search query language").required(true)),
        )
        .subcommand(
            Command::new("ip")
                .about("Search based on IP address")
                .arg_required_else_help(true)
                .arg(arg!([address] "IP address").required(true)),
        )
        .subcommand(
            Command::new("dns")
                .about("Search based on DNS name")
                .arg_required_else_help(true)
                .arg(arg!([dns_name] "DNS name").required(true)),
        )
        .subcommand(
            Command::new("asn")
                .about("Search based on autonomous system number")
                .arg_required_else_help(true)
                .arg(arg!([asn] "Autonomous system number").required(true)),
        )
        .subcommand(
            Command::new("cert")
                .about("Search based on TLS certificate")
                .arg_required_else_help(true)
                .subcommand(
                    Command::new("hosts")
                        .about("Search for hosts related to the certificate")
                        .arg_required_else_help(true)
                        .arg(
                            arg!([fingerprint] "SHA256 fingerprint of the certificate")
                                .required(true),
                        ),
                )
                .subcommand(
                    Command::new("comments")
                        .about("Search for comments related to the certificate")
                        .arg_required_else_help(true)
                        .arg(
                            arg!([fingerprint] "SHA256 fingerprint of the certificate")
                                .required(true),
                        ),
                ),
        )
        .subcommand(
            Command::new("fields").about("Show all available Censys Search query language fields"),
        )
        .get_matches();

    let api_id = match arg_matches.get_one::<String>("api_id") {
        Some(value) => value.to_owned(),
        None => get_env_or_exit("CENSYS_API_ID"),
    };
    let secret = match arg_matches.get_one::<String>("secret") {
        Some(value) => value.to_owned(),
        None => get_env_or_exit("CENSYS_SECRET"),
    };
    let no_paging = *arg_matches
        .get_one::<bool>("no_paging")
        .expect("Argument always has a value");
    let out_file = arg_matches.get_one::<String>("output").map(Path::new);
    let token = base64::encode(format!("{}:{}", api_id, secret));
    let client = Client::new();

    match arg_matches.subcommand() {
        Some(("query", query_command)) => {
            let query = query_command
                .get_one::<String>("query")
                .expect("Argument is required");
            let path = make_path_from_query(query);
            output_response(&client, &token, &path, no_paging, out_file);
        }
        Some(("ip", ip_command)) => {
            let address = ip_command
                .get_one::<String>("address")
                .expect("Argument is required");
            let path = make_path_from_ip(address);
            output_response(&client, &token, &path, no_paging, out_file);
        }
        Some(("dns", dns_command)) => {
            let dns_name = dns_command
                .get_one::<String>("dns_name")
                .expect("Argument is required");
            let query = format!("dns.names: {}", dns_name);
            let path = make_path_from_query(&query);
            output_response(&client, &token, &path, no_paging, out_file);
        }
        Some(("asn", asn_command)) => {
            let asn = asn_command
                .get_one::<String>("asn")
                .expect("Argument is required");
            let query = format!("autonomous_system.asn: {}", asn);
            let path = make_path_from_query(&query);
            output_response(&client, &token, &path, no_paging, out_file);
        }
        Some(("cert", cert_command)) => match cert_command.subcommand() {
            Some(("hosts", hosts_command)) => {
                let fingerprint = hosts_command
                    .get_one::<String>("fingerprint")
                    .expect("Argument is required");
                let path = make_hosts_path_from_cert_fingerprint(fingerprint);
                output_response(&client, &token, &path, no_paging, out_file);
            }
            Some(("comments", comments_command)) => {
                let fingerprint = comments_command
                    .get_one::<String>("fingerprint")
                    .expect("Argument is required");
                let path = make_comments_path_from_cert_fingerprint(fingerprint);
                output_response(&client, &token, &path, no_paging, out_file);
            }
            _ => unreachable!("All subcommands exhausted"),
        },
        Some(("fields", _)) => {
            println!("{}", constants::CENSYS_SEARCH_FIELDS);
        }
        _ => unreachable!("All subcommands exhausted"),
    }
}

fn output_response(
    client: &Client,
    token: &str,
    path: &str,
    no_paging: bool,
    out_file: Option<&Path>,
) {
    let mut json_response = send_request(client, token, path);
    write_to_file_or_std_out(&json_response, out_file);
    if no_paging {
        return;
    }
    let mut cursor = get_cursor_from_response(&json_response);
    while cursor.is_some() {
        let path = format!("{}&cursor={}", path, cursor.unwrap());
        json_response = send_request(client, token, &path);
        write_to_file_or_std_out(&json_response, out_file);
        cursor = get_cursor_from_response(&json_response);
    }
}

fn write_to_file_or_std_out(json: &Value, out_file: Option<&Path>) {
    match out_file {
        Some(path) => {
            let mut file = fs::OpenOptions::new()
                .create(true)
                .write(true)
                .append(true)
                .open(path)
                .expect("Unable to open file");
            writeln!(file, "{}", json).expect("Unable to write to file");
        }
        None => println!("{}", json),
    }
}

fn get_cursor_from_response(json_response: &Value) -> Option<String> {
    let cursor = &json_response["result"]["links"]["next"];
    match cursor {
        Value::String(value) if !value.is_empty() => Some(value.to_owned()),
        _ => None,
    }
}

fn make_path_from_query(query: &str) -> String {
    let query = urlencoding::encode(query).into_owned();
    format!("/hosts/search?q={}", query)
}

fn make_hosts_path_from_cert_fingerprint(fingerprint: &str) -> String {
    format!("/certificates/{}/hosts", fingerprint)
}

fn make_comments_path_from_cert_fingerprint(fingerprint: &str) -> String {
    format!("/certificates/{}/comments", fingerprint)
}

fn make_path_from_ip(ip: &str) -> String {
    format!("/hosts/{}", ip)
}

fn send_request(client: &Client, token: &str, path: &str) -> Value {
    let response = client
        .get(format!("{}{}", constants::BASE_URL, path))
        .header(ACCEPT, "application/json")
        .header(AUTHORIZATION, format!("Basic {}", token))
        .send();
    match response {
        Ok(resp) => resp.json().unwrap(),
        Err(err) => {
            eprintln!("{}", err);
            process::exit(1);
        }
    }
}

fn get_env_or_exit(name: &str) -> String {
    env::var(name).unwrap_or_else(|_| {
        eprintln!("{} is not defined", name);
        process::exit(1);
    })
}
