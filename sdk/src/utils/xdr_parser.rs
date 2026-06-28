use stellar_xdr::curr::{
    HostFunction, InvokeContractArgs, InvokeHostFunctionOp, Operation, OperationBody, ReadXdr,
    ScAddress, ScVal, TransactionEnvelope, TransactionV1Envelope,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("XDR decode error: {0}")]
    Xdr(String),
    #[error("Not a Soroban invoke operation")]
    NotSorobanInvoke,
    #[error("Missing invoke host function")]
    MissingInvokeHostFunction,
}

#[derive(Debug, Clone)]
pub struct SorobanInvocation {
    /// Strkey-encoded contract address (C...) or account ID (G...).
    pub contract_id: String,
    pub function_name: String,
    pub arguments: Vec<ScVal>,
}

/// Parse a base64-encoded `TransactionEnvelope` XDR and extract the Soroban invocation.
pub fn parse_soroban_invoke(xdr: &str) -> Result<SorobanInvocation, ParseError> {
    let envelope = TransactionEnvelope::from_xdr_base64(xdr, stellar_xdr::curr::Limits::none())
        .map_err(|e| ParseError::Xdr(e.to_string()))?;

    let ops: &[Operation] = match &envelope {
        TransactionEnvelope::Tx(TransactionV1Envelope { tx, .. }) => &tx.operations,
        _ => return Err(ParseError::NotSorobanInvoke),
    };

    let invoke_op: &InvokeHostFunctionOp = ops
        .iter()
        .find_map(|op| match &op.body {
            OperationBody::InvokeHostFunction(ihf) => Some(ihf),
            _ => None,
        })
        .ok_or(ParseError::MissingInvokeHostFunction)?;

    let InvokeContractArgs {
        contract_address,
        function_name,
        args,
    } = match &invoke_op.host_function {
        HostFunction::InvokeContract(a) => a,
        _ => return Err(ParseError::NotSorobanInvoke),
    };

    let contract_id = match contract_address {
        ScAddress::Contract(hash) => stellar_strkey::Contract(hash.0).to_string(),
        ScAddress::Account(account_id) => {
            use stellar_xdr::curr::PublicKey;
            match &account_id.0 {
                PublicKey::PublicKeyTypeEd25519(key) => {
                    stellar_strkey::ed25519::PublicKey(key.0).to_string()
                }
            }
        }
    };

    Ok(SorobanInvocation {
        contract_id,
        function_name: String::from_utf8_lossy(function_name.as_slice()).into_owned(),
        arguments: args.to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_xdr_returns_parse_error() {
        let result = parse_soroban_invoke("not-valid-xdr!!!");
        assert!(matches!(result, Err(ParseError::Xdr(_))));
    }

    #[test]
    fn non_invoke_tx_returns_error() {
        // Valid base64 but not valid XDR TransactionEnvelope
        let result = parse_soroban_invoke("AAAAAA==");
        assert!(result.is_err());
    }
}
