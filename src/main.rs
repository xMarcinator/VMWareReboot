use std::{env, fmt};
use std::fs::File;
use std::io::BufReader;
use std::net::Ipv4Addr;
use std::time::Duration;
use reqwest::{ClientBuilder, Error, header, Response, Url};
use reqwest::header::HeaderMap;

use base64::prelude::*;
use clap::{Args, Parser, Subcommand};
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use serde_qs as qs;

struct VMWare {
    session_id:String,
    baseAddress:String,
    client:reqwest::Client
}



impl VMWare {
    async fn connect(ip:&Ipv4Addr,user:&str,password:&str) -> Result<VMWare, ()>{
        let base = format!("https://{}",ip.to_string());

        let session_id = Self::authenticate(&base,user,password).await;


        let mut headers = HeaderMap::new();

        headers.insert("vmware-api-session-id",header::HeaderValue::from_str(&*session_id).unwrap());


        let builder = reqwest::Client::builder().default_headers(headers);

        let client = match builder.build() {
            Ok(client) => {client}
            Err(_) => {return Err(())}
        };


        Ok(VMWare {
            baseAddress:base,
            session_id,
            client
        })
    }

    async fn authenticate(base:&str, user:&str, password:&str) -> String {
        let url = format!("{}/api/session",base);

        let token = BASE64_STANDARD.encode(format!("{}:{}",user,password));

        reqwest::Client::new()
            .get(url)
            .header("Authorization", format!("Basic {}",token))
            .timeout(Duration::from_micros(1000))
            .send()
            .await.unwrap()
            .text()
            .await.unwrap()
    }

    async fn list_vms(&self) -> Result<Vec<VMSummary>,()>{
        let url = self.baseAddress.clone() + "{}/api/vcenter/vm";

        let res = match self.client.get(url).send().await {
            Ok(resp) => {
                resp
            }
            Err(_) => {return Err(())}
        };

        match res.json().await {
            Ok(v) => Ok(v),
            Err(_) => Err(())
        }
    }

    async fn list_vms_options(&self,options:&VMListOptions) -> Result<Vec<VMSummary>,()>{
        let parameters =  qs::to_string(options).expect("Failed to build query");

        let url = format!("{}/api/vcenter/vm?{}",self.baseAddress.clone(),parameters);

        let res = match self.client.get(url).send().await {
            Ok(resp) => {
                resp
            }
            Err(_) => {return Err(())}
        };

        match res.json().await {
            Ok(v) => Ok(v),
            Err(_) => Err(())
        }
    }



    async fn list_selected_vms(&self,vms:&[&str]) -> Result<Vec<VMSummary>,()>{
        let url = self.baseAddress.clone() + "{}/api/vcenter/vm";

        let res = match self.client.get(url).send().await {
            Ok(resp) => {
                resp
            }
            Err(_) => {return Err(())}
        };

        match res.json().await {
            Ok(v) => Ok(v),
            Err(_) => Err(())
        }
    }

    async fn shutdown_vm_guest(&self,vm:&str) -> Result<Vec<VMSummary>,()>{
        return self.power_action_vm_guest(vm,VmPowerAction::shutdown).await
    }

    async fn reboot_vm_guest(&self,vm:&str) -> Result<Vec<VMSummary>,()>{
        return self.power_action_vm_guest(vm,VmPowerAction::reboot).await
    }

    async fn standby_vm_guest(&self,vm:&str) -> Result<Vec<VMSummary>,()>{
        return self.power_action_vm_guest(vm,VmPowerAction::standby).await
    }

    async fn power_action_vm_guest(&self,vm:&str,action:VmPowerAction) -> Result<Vec<VMSummary>,()>{
        let url = format!("{}/api/vcenter/vm/{}/guest/power?action={}",self.baseAddress,vm,action.to_string());

        let res = match self.client.get(url).send().await {
            Ok(resp) => {
                resp
            }
            Err(_) => {return Err(())}
        };

        match res.json().await {
            Ok(v) => Ok(v),
            Err(_) => Err(())
        }
    }
}

#[derive(Debug, PartialEq, Deserialize, Serialize,Default)]
struct VMListOptions where {
    clusters:Option<Vec<String>>,
    datacenters:Option<Vec<String>>,
    folders:Option<Vec<String>>,
    hosts:Option<Vec<String>>,
    names:Option<Vec<String>>,
    power_states:Option<Vec<VmPowerState>>,
    resource_pools:Option<Vec<String>>,
    vms:Option<Vec<String>>,
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
struct VMSummary{
    name: String,
    power_state: VmPowerState,
    #[serde(rename="vm")]
    id:String,
    cpu_count:Option<i64>,
    memory_size_mib:Option<i64>
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
enum VmPowerState{
    POWERED_OFF,
    POWERED_ON,
    SUSPENDED
}

#[derive(Debug,Deserialize, Serialize)]
enum VmPowerAction{
    shutdown,
    reboot,
    standby
}

impl fmt::Display for VmPowerAction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
        // or, alternatively:
        // fmt::Debug::fmt(self, f)
    }
}


#[derive(Debug,Clone)]
enum RunMode {
    Start,
    Shutdown,
    Auto
}



/// Simple program to greet a person
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// File location of a config file
    config: Option<std::path::PathBuf>,

    /// Name of the person to greet
    #[arg(short, long)]
    vm_only: bool,

    /// Should order of vms be ignored
    #[arg(short, long)]
    ignore_order: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Start vms
    Start,
    /// Shutdown vms
    Shutdown,
    /// Auto detect run mode vms
    Auto
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
struct Config{
    ip:Ipv4Addr
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let args = Cli::parse();

    let username = std::env::var("VCENTER_USERNAME").expect("VCENTER_USERNAME must be set.");
    let password = std::env::var("VCENTER_PASSWORD").expect("VCENTER_PASSWORD must be set.");

    let mut configPath = args.config.unwrap_or_else(||{
        env::current_dir().unwrap()
    });

    if (configPath.is_dir()){
        configPath = configPath.join("config.json");
    }

    println!("The current directory is {}", configPath.display());

    let file = File::open(configPath).expect("Unable to open file");
    let reader = BufReader::new(file);




    // Read the JSON contents of the file as an instance of `User`.
    let config:Config = serde_json::from_reader(reader).unwrap();


    let host = match VMWare::connect(&config.ip,&username,&password).await{
        Ok(v) => v,
        Err(_) => {panic!()}
    };

    match &args.command {
        Commands::Start => {
            start(host,config).await;
        }
        Commands::Shutdown => {
            stop(host,config).await;
        }
        Commands::Auto => {
            todo!()
        }
    }
}

fn thing() {
    todo!()
}

async fn start(host:VMWare,args:Config){
    todo!();
}

async fn stop(host:VMWare,args:Config){
    todo!();

    for vm in host.list_vms().await.unwrap() {
        println!("vm {} is in state {:?}",vm.name,vm.power_state);

        match vm.power_state {
            VmPowerState::POWERED_OFF => {
                host.shutdown_vm_guest(&vm.id).await.unwrap();
            }
            VmPowerState::POWERED_ON => {}
            VmPowerState::SUSPENDED => {}
        }
    }
}
