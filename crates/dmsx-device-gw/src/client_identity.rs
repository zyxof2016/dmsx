//! 从 mTLS 客户端证书 SAN（URI）解析 `urn:dmsx:tenant:{uuid}:device:{uuid}`，与 RPC 中的租户/设备声明对齐。

use tonic::{Request, Status};
use uuid::Uuid;
use x509_parser::extensions::GeneralName;
use x509_parser::extensions::ParsedExtension;
use x509_parser::prelude::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClientIdentity {
    pub tenant_id: Uuid,
    pub device_id: Uuid,
}

fn parse_dmsx_urn(uri: &str) -> Option<(Uuid, Uuid)> {
    const PREFIX: &str = "urn:dmsx:tenant:";
    let rest = uri.strip_prefix(PREFIX)?;
    let (tenant_part, device_part) = rest.split_once(":device:")?;
    let tid = Uuid::parse_str(tenant_part).ok()?;
    let did = Uuid::parse_str(device_part).ok()?;
    Some((tid, did))
}

pub fn identity_from_peer_certs_der(certs: &[impl AsRef<[u8]>]) -> Result<ClientIdentity, Status> {
    let leaf = certs
        .first()
        .ok_or_else(|| Status::unauthenticated("mTLS: empty certificate chain"))?;
    let (_, x509) = X509Certificate::from_der(leaf.as_ref())
        .map_err(|_| Status::unauthenticated("mTLS: invalid X.509 certificate"))?;

    for ext in x509.extensions() {
        if let ParsedExtension::SubjectAlternativeName(san) = ext.parsed_extension() {
            for gn in san.general_names.iter() {
                if let GeneralName::URI(uri) = gn {
                    if let Some((tid, did)) = parse_dmsx_urn(uri) {
                        return Ok(ClientIdentity {
                            tenant_id: tid,
                            device_id: did,
                        });
                    }
                }
            }
        }
    }

    Err(Status::failed_precondition(
        "mTLS: client certificate SAN must include URI urn:dmsx:tenant:{uuid}:device:{uuid}",
    ))
}

/// `require_mtls_identity`：已启用服务端 TLS 且配置了客户端 CA（强制校验客户端证书）时为 true。
pub fn resolve_tenant_device<T>(
    request: &Request<T>,
    require_mtls_identity: bool,
    proto_tenant_id: &str,
    proto_device_id: &str,
) -> Result<(Uuid, Uuid), Status> {
    let did = Uuid::parse_str(proto_device_id.trim())
        .map_err(|_| Status::invalid_argument("device_id must be a UUID"))?;

    if require_mtls_identity {
        let certs = request
            .peer_certs()
            .ok_or_else(|| Status::unauthenticated("mTLS: peer certificates not available"))?;
        let id = identity_from_peer_certs_der(certs.as_ref().as_slice())?;
        if id.device_id != did {
            return Err(Status::permission_denied(
                "device_id does not match client certificate",
            ));
        }
        if proto_tenant_id.trim().is_empty() {
            return Ok((id.tenant_id, id.device_id));
        }
        let tid = Uuid::parse_str(proto_tenant_id.trim())
            .map_err(|_| Status::invalid_argument("tenant_id must be a UUID"))?;
        if tid != id.tenant_id {
            return Err(Status::permission_denied(
                "tenant_id does not match client certificate",
            ));
        }
        return Ok((id.tenant_id, id.device_id));
    }

    if proto_tenant_id.trim().is_empty() {
        return Err(Status::invalid_argument(
            "tenant_id is required when mTLS identity enforcement is disabled",
        ));
    }
    let tid = Uuid::parse_str(proto_tenant_id.trim())
        .map_err(|_| Status::invalid_argument("tenant_id must be a UUID"))?;
    Ok((tid, did))
}

/// 仅校验 `device_id`（如 Heartbeat）；mTLS 开启时须与证书一致。
pub fn resolve_device_only<T>(
    request: &Request<T>,
    require_mtls_identity: bool,
    proto_device_id: &str,
) -> Result<Uuid, Status> {
    let did = Uuid::parse_str(proto_device_id.trim())
        .map_err(|_| Status::invalid_argument("device_id must be a UUID"))?;
    if !require_mtls_identity {
        return Ok(did);
    }
    let certs = request
        .peer_certs()
        .ok_or_else(|| Status::unauthenticated("mTLS: peer certificates not available"))?;
    let id = identity_from_peer_certs_der(certs.as_ref().as_slice())?;
    if id.device_id != did {
        return Err(Status::permission_denied(
            "device_id does not match client certificate",
        ));
    }
    Ok(did)
}
