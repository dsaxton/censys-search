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
        .about("Wrapper for the Censys Search API")
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
                .arg(arg!([address]).required(true)),
        )
        .subcommand(
            Command::new("query")
                .about("Search based on query")
                .arg(arg!([query]).required(true)),
        )
        .get_matches();

    let api_id = if let Some(value) = matches.get_one::<String>("api_id") {
        value.to_owned()
    } else {
        get_env_or_exit("CENSYS_API_ID")
    };
    let secret = if let Some(value) = matches.get_one::<String>("secret") {
        value.to_owned()
    } else {
        get_env_or_exit("CENSYS_SECRET")
    };
    let token = base64::encode(format!("{}:{}", api_id, secret));
    let client = Client::new();

    match matches.subcommand() {
        Some(("ip", sub_match)) => {
            let address = sub_match
                .get_one::<String>("address")
                .expect("Argument is required");
            let uri = make_uri_from_ip(&address);
            let json_response = send_request(&client, &token, &uri);
            println!("{}", json_response);
        }
        Some(("query", sub_match)) => {
            let query = sub_match
                .get_one::<String>("query")
                .expect("Argument is required");
            let uri = make_uri_from_query(&query);
            let json_response = send_request(&client, &token, &uri);
            println!("{}", json_response);
        }
        _ => unreachable!("All subcommands exhausted"),
    }
}

fn make_uri_from_query(query: &str) -> String {
    let query = urlencoding::encode(&query).into_owned();
    format!("/hosts/search?q={}", query)
}

fn make_uri_from_ip(ip: &str) -> String {
    let ip = urlencoding::encode(ip).into_owned();
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
