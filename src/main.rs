use std::net::Ipv4Addr;
use reqwest::{ClientBuilder, Error, header, Response};
use reqwest::header::HeaderMap;

use base64::prelude::*;
use serde::{Deserialize, Serialize};

struct VMWare {
    session_id:String,
    baseAddress:String,
    client:reqwest::Client
}

impl VMWare {
    async fn connect(ip:Ipv4Addr,user:&str,password:&str) -> Result<VMWare, ()>{
        let base = format!("https://{}",ip.to_string());

        let session_id = Self::authenticate(&base,user,password).await;


        let mut headers = HeaderMap::new();

        headers.insert("vmware-api-session-id",header::HeaderValue::from(&session_id));


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

    async fn authenticate(base:&str,user:&str,password:&str) -> String {
        let url = format!("{}/api/session",base);

        let token = BASE64_STANDARD.encode(format!("{}:{}",user,password));

        reqwest::Client::new()
            .get(url)
            .header("Authorization", format!("Basic {}",token))
            .send()
            .await?
            .text()
            .await?
    }

    async fn list_vms(&self) -> Result<Vec<VMSummary>,()>{
        let url = self.baseAddress.clone() + "{}/api/vcenter/vm";

        let res = match self.client.get(url).send().await {
            Ok(resp) => {
                resp
            }
            Err(_) => {}
        };

        match res.json().await {
            Some(v) => Ok(v),
            Err(_) => Err(())
        }
    }
}

struct VMSummary{
    name: String,
    power_state: VmPowerState,
    vm:String,
    cpu_count:Option<i64>,
    memory_size_mib:Option<i64>
}

enum VmPowerState{
    POWERED_OFF,
    POWERED_ON,
    SUSPENDED
}


#[tokio::main]
fn main() {
    VMWare::connect(Ipv4Addr::LOCALHOST,"")
}
