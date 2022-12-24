use clap::{arg, Command};
use reqwest::blocking::Client;
use reqwest::header::{ACCEPT, AUTHORIZATION};
use serde_json::Value;
use std::{env, process};

// services.service_name: elasticsearch and services.http.response.headers.status: 200

const BASE_URL: &str = "https://search.censys.io/api/v2";

fn main() {
    let matches = Command::new("censys-search")
        .version("1.0")
        .about("Censys Search API wrapper utility")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .arg(
            arg!(--api_id <VALUE> "API ID (if not specified CENSYS_API_ID must be set)")
                .required(false),
        )
        .arg(
            arg!(--secret <VALUE> "API secret (if not specified CENSYS_SECRET must be set)")
                .required(false),
        )
        .subcommand(
            Command::new("ip")
                .about("Search based on IP address")
                .arg_required_else_help(true)
                .arg(arg!([address] "IP address").required(true)),
        )
        .subcommand(
            Command::new("query")
                .about("Search based on query")
                .arg_required_else_help(true)
                .arg(arg!([query] "Query using the Censys Search query language").required(true)),
        )
        .subcommand(
            Command::new("cert")
                .about("Search based on TLS certificate")
                .arg_required_else_help(true)
                .subcommand(
                    Command::new("hosts")
                        .about("Search for hosts")
                        .arg_required_else_help(true)
                        .arg(
                            arg!([fingerprint] "SHA256 fingerprint of the certificate")
                                .required(true),
                        ),
                )
                .subcommand(
                    Command::new("comments")
                        .about("Search for comments")
                        .arg_required_else_help(true)
                        .arg(
                            arg!([fingerprint] "SHA256 fingerprint of the certificate")
                                .required(true),
                        ),
                ),
        )
        .get_matches();

    let api_id = match matches.get_one::<String>("api_id") {
        Some(value) => value.to_owned(),
        None => get_env_or_exit("CENSYS_API_ID"),
    };
    let secret = match matches.get_one::<String>("secret") {
        Some(value) => value.to_owned(),
        None => get_env_or_exit("CENSYS_SECRET"),
    };
    let token = base64::encode(format!("{}:{}", api_id, secret));
    let client = Client::new();

    match matches.subcommand() {
        Some(("ip", ip_match)) => {
            let address = ip_match
                .get_one::<String>("address")
                .expect("Argument is required");
            let uri = make_uri_from_ip(address);
            let json_response = send_request(&client, &token, &uri);
            println!("{}", json_response);
        }
        Some(("query", query_match)) => {
            let query = query_match
                .get_one::<String>("query")
                .expect("Argument is required");
            let uri = make_uri_from_query(query);
            let json_response = send_request(&client, &token, &uri);
            println!("{}", json_response);
        }
        Some(("cert", cert_match)) => match cert_match.subcommand() {
            Some(("hosts", hosts_match)) => {
                let fingerprint = hosts_match
                    .get_one::<String>("fingerprint")
                    .expect("Argument is required");
                let uri = make_hosts_uri_from_cert_fingerprint(fingerprint);
                let json_response = send_request(&client, &token, &uri);
                println!("{}", json_response);
            }
            Some(("comments", comments_match)) => {
                let fingerprint = comments_match
                    .get_one::<String>("fingerprint")
                    .expect("Argument is required");
                let uri = make_comments_uri_from_cert_fingerprint(fingerprint);
                let json_response = send_request(&client, &token, &uri);
                println!("{}", json_response);
            }
            _ => unreachable!("All subcommands exhausted"),
        },
        _ => unreachable!("All subcommands exhausted"),
    }
}

fn make_uri_from_query(query: &str) -> String {
    let query = urlencoding::encode(query).into_owned();
    format!("/hosts/search?q={}", query)
}

fn make_hosts_uri_from_cert_fingerprint(fingerprint: &str) -> String {
    format!("/certificates/{}/hosts", fingerprint)
}

fn make_comments_uri_from_cert_fingerprint(fingerprint: &str) -> String {
    format!("/certificates/{}/comments", fingerprint)
}

fn make_uri_from_ip(ip: &str) -> String {
    let ip = urlencoding::encode(ip).to_string();
    format!("/hosts/{}", ip)
}

fn send_request(client: &Client, token: &str, uri: &str) -> Value {
    let response = client
        .get(format!("{}{}", BASE_URL, uri))
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
