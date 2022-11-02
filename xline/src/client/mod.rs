use std::net::SocketAddr;

// use anyhow::{anyhow, Result};
use curp::{client::Client as CurpClient, cmd::ProposeId};
use etcd_client::Client as EtcdClient;
use prost::Message;
use uuid::Uuid;

use crate::{
    rpc::{DeleteRangeResponse, PutResponse, RangeResponse, Request, RequestOp, Response},
    server::command::{Command, KeyRange},
};

use kv_types::{PutRequest, RangeRequest};

use self::{errors::ClientError, kv_types::DeleteRangeRequest};

/// covert struct between etcd and curp
mod convert;
/// Error types
pub mod errors;
/// Requests used by Client
pub mod kv_types;

/// Xline client
#[allow(missing_debug_implementations)] // EtcdClient doesn't implement Debug
pub struct Client {
    /// Name of the client
    name: String,
    /// Curp client
    curp_client: CurpClient<Command>,
    /// Etcd client
    etcd_client: EtcdClient,
    /// Use curp client to send requests when true
    use_curp_client: bool,
}

impl Client {
    /// New `Client`
    ///
    /// # Errors
    ///
    /// If `EtcdClient::connect` fails.
    #[inline]
    pub async fn new(
        leader_index: usize,
        all_members: Vec<SocketAddr>,
        use_curp_client: bool,
    ) -> Result<Self, ClientError> {
        let etcd_client = EtcdClient::connect(
            all_members
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            None,
        )
        .await?;
        let curp_client = CurpClient::new(leader_index, all_members).await;
        Ok(Self {
            name: String::from("client"),
            curp_client,
            etcd_client,
            use_curp_client,
        })
    }

    /// set `use_curp_client`
    #[inline]
    pub fn set_use_curp_client(&mut self, use_curp_client: bool) {
        self.use_curp_client = use_curp_client;
    }

    /// Generate a new `ProposeId`
    fn generate_propose_id(&self) -> ProposeId {
        ProposeId::new(format!("{}-{}", self.name, Uuid::new_v4()))
    }

    /// Send `PutRequest` by `CurpClient` or `EtcdClient`
    ///
    /// # Errors
    ///
    /// If `CurpClient` or `EtcdClient` failed to send request
    #[inline]
    pub async fn put(&mut self, request: PutRequest) -> Result<PutResponse, ClientError> {
        if self.use_curp_client {
            let key_ranges = vec![KeyRange {
                start: request.key().to_vec(),
                end: vec![],
            }];
            let req_op = RequestOp {
                request: Some(Request::RequestPut(request.into())),
            };
            let propose_id = self.generate_propose_id();
            let cmd = Command::new(key_ranges, req_op.encode_to_vec(), propose_id);
            let cmd_res = self.curp_client.propose(cmd).await?;
            if let Some(Response::ResponsePut(response)) = cmd_res.decode().response {
                Ok(response)
            } else {
                Err(ClientError::ParseError(String::from(
                    "PutResponse parse error",
                )))
            }
        } else {
            let opts = (&request).into();
            let response = self
                .etcd_client
                .put(request.key(), request.value(), Some(opts))
                .await?;
            Ok(response.into())
        }
    }

    /// Send `RangeRequest` by `CurpClient` or `EtcdClient`
    ///
    /// # Errors
    ///
    /// If `CurpClient` or `EtcdClient` failed to send request
    #[inline]
    pub async fn range(&mut self, request: RangeRequest) -> Result<RangeResponse, ClientError> {
        if self.use_curp_client {
            let key_ranges = vec![KeyRange {
                start: request.key().to_vec(),
                end: request.range_end().to_vec(),
            }];
            let req_op = RequestOp {
                request: Some(Request::RequestRange(request.into())),
            };
            let propose_id = self.generate_propose_id();
            let cmd = Command::new(key_ranges, req_op.encode_to_vec(), propose_id);
            let cmd_res = self.curp_client.propose(cmd).await?;
            if let Some(Response::ResponseRange(response)) = cmd_res.decode().response {
                Ok(response)
            } else {
                Err(ClientError::ParseError(String::from(
                    "RangeResponse parse error",
                )))
            }
        } else {
            let opts = (&request).into();
            let response = self.etcd_client.get(request.key(), Some(opts)).await?;
            Ok(response.into())
        }
    }

    /// Send `DeleteRangeRequest` by `CurpClient` or `EtcdClient`
    ///
    /// # Errors
    ///
    /// If `CurpClient` or `EtcdClient` failed to send request
    #[inline]
    pub async fn delete(
        &mut self,
        request: DeleteRangeRequest,
    ) -> Result<DeleteRangeResponse, ClientError> {
        if self.use_curp_client {
            let key_ranges = vec![KeyRange {
                start: request.key().to_vec(),
                end: request.range_end().to_vec(),
            }];
            let req_op = RequestOp {
                request: Some(Request::RequestDeleteRange(request.into())),
            };
            let propose_id = self.generate_propose_id();
            let cmd = Command::new(key_ranges, req_op.encode_to_vec(), propose_id);
            let cmd_res = self.curp_client.propose(cmd).await?;
            if let Some(Response::ResponseDeleteRange(response)) = cmd_res.decode().response {
                Ok(response)
            } else {
                Err(ClientError::ParseError(String::from(
                    "DeleteRangeResponse parse error",
                )))
            }
        } else {
            let opts = (&request).into();
            let response = self.etcd_client.delete(request.key(), Some(opts)).await?;
            Ok(response.into())
        }
    }
}