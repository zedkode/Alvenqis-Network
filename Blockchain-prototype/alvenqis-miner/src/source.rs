use crate::{
    MinerError, MiningSubmitRequest, MiningSubmitResponse, MiningTemplate, Result, SubmitStatus,
    MINING_PROTOCOL_VERSION,
};
use alvenqis_core::{Address, Block};
use reqwest::blocking::Client;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

pub trait WorkSource {
    fn fetch_template(&self, miner_address: &str) -> Result<MiningTemplate>;
    fn submit(&self, request: &MiningSubmitRequest) -> Result<MiningSubmitResponse>;
    fn description(&self) -> String;
    fn validate_and_build(&self, template: &MiningTemplate, miner_address: &str) -> Result<Block> {
        template.validate_and_build(miner_address)
    }
}

pub struct RpcWorkSource {
    base_url: String,
    client: Client,
}

pub struct PoolWorkSource {
    base_url: String,
    miner_address: String,
    worker_name: String,
    client: Client,
}

impl PoolWorkSource {
    pub fn new(
        base_url: String,
        miner_address: String,
        worker_name: String,
        timeout: Duration,
    ) -> Result<Self> {
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|error| MinerError::Rpc(error.to_string()))?;
        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_owned(),
            miner_address,
            worker_name,
            client,
        })
    }
}

impl WorkSource for PoolWorkSource {
    fn fetch_template(&self, miner_address: &str) -> Result<MiningTemplate> {
        let response = self
            .client
            .get(format!("{}/api/v1/work", self.base_url))
            .query(&[
                ("miner_address", miner_address),
                ("worker_name", &self.worker_name),
            ])
            .send()
            .map_err(|error| MinerError::Rpc(error.to_string()))?;
        parse_response(response, "pool work")
    }

    fn submit(&self, request: &MiningSubmitRequest) -> Result<MiningSubmitResponse> {
        let mut request = request.clone();
        request.miner_address = Some(self.miner_address.clone());
        request.worker_name = Some(self.worker_name.clone());
        let response = self
            .client
            .post(format!("{}/api/v1/shares", self.base_url))
            .json(&request)
            .send()
            .map_err(|error| MinerError::Rpc(error.to_string()))?;
        parse_response(response, "pool share")
    }

    fn description(&self) -> String {
        format!("pool {} worker={}", self.base_url, self.worker_name)
    }

    fn validate_and_build(&self, template: &MiningTemplate, _miner_address: &str) -> Result<Block> {
        let reward_address = template
            .transactions
            .first()
            .map(|transaction| transaction.to.as_str())
            .ok_or_else(|| MinerError::InvalidTemplate("coinbase is missing".to_owned()))?;
        Address::parse(reward_address)?;
        template.validate_and_build(reward_address)
    }
}

impl RpcWorkSource {
    pub fn new(base_url: String, timeout: Duration) -> Result<Self> {
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|error| MinerError::Rpc(error.to_string()))?;
        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_owned(),
            client,
        })
    }
}

impl WorkSource for RpcWorkSource {
    fn fetch_template(&self, miner_address: &str) -> Result<MiningTemplate> {
        let response = self
            .client
            .get(format!("{}/mining/template", self.base_url))
            .query(&[("miner_address", miner_address)])
            .send()
            .map_err(|error| MinerError::Rpc(error.to_string()))?;
        parse_response(response, "template")
    }

    fn submit(&self, request: &MiningSubmitRequest) -> Result<MiningSubmitResponse> {
        let response = self
            .client
            .post(format!("{}/mining/submit", self.base_url))
            .json(request)
            .send()
            .map_err(|error| MinerError::Rpc(error.to_string()))?;
        parse_response(response, "submission")
    }

    fn description(&self) -> String {
        format!("RPC {}", self.base_url)
    }
}

pub struct FileWorkSource {
    template_path: PathBuf,
    submission_path: PathBuf,
}

impl FileWorkSource {
    pub fn new(template_path: PathBuf, submission_path: PathBuf) -> Self {
        Self {
            template_path,
            submission_path,
        }
    }
}

impl WorkSource for FileWorkSource {
    fn fetch_template(&self, _miner_address: &str) -> Result<MiningTemplate> {
        if !self.template_path.is_file() {
            return Err(MinerError::WorkFileMissing(self.template_path.clone()));
        }
        let content = fs::read_to_string(&self.template_path)?;
        Ok(serde_json::from_str(&content)?)
    }

    fn submit(&self, request: &MiningSubmitRequest) -> Result<MiningSubmitResponse> {
        if let Some(parent) = self.submission_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let temporary = self.submission_path.with_extension("json.tmp");
        fs::write(&temporary, serde_json::to_vec_pretty(request)?)?;
        fs::rename(&temporary, &self.submission_path)?;
        Ok(MiningSubmitResponse {
            protocol: MINING_PROTOCOL_VERSION.to_owned(),
            status: SubmitStatus::PendingLocal,
            template_id: request.template_id.clone(),
            block_hash: request.block_hash.clone(),
            height: None,
            reason: Some("written to local submission file; node acceptance is pending".to_owned()),
        })
    }

    fn description(&self) -> String {
        format!("local file {}", self.template_path.display())
    }
}

fn parse_response<T: serde::de::DeserializeOwned>(
    response: reqwest::blocking::Response,
    operation: &str,
) -> Result<T> {
    let status = response.status();
    let body = response
        .text()
        .map_err(|error| MinerError::Rpc(error.to_string()))?;
    if !status.is_success() {
        return Err(MinerError::Rpc(format!(
            "{operation} returned HTTP {status}: {body}"
        )));
    }
    serde_json::from_str(&body)
        .map_err(|error| MinerError::Rpc(format!("invalid {operation} JSON: {error}")))
}
