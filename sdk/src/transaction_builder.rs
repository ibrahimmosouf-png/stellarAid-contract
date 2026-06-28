use crate::errors::{Result, StellarAidError};
use crate::horizon::client::HorizonClient;
use crate::soroban::rpc_client::SorobanRpcClient;

/// Network configuration for building Soroban transactions.
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    pub rpc_url: String,
    pub horizon_url: String,
    pub network_passphrase: String,
}

/// Build a `donate()` Soroban transaction XDR that can be signed by a wallet client.
///
/// Returns a base64-encoded `TransactionEnvelope` XDR ready for client-side signing.
pub async fn build_donate_transaction(
    donor: &str,
    campaign_id: u64,
    amount: i128,
    network: &NetworkConfig,
    donation_contract_id: &str,
) -> Result<String> {
    use soroban_sdk::xdr::{
        AccountId, Hash, HostFunction, InvokeHostFunctionOp, Memo, MuxedAccount,
        Operation, OperationBody, Preconditions, PublicKey, ScAddress, ScVal,
        ScVec, SequenceNumber, TimeBounds, Transaction, TransactionEnvelope,
        TransactionExt, Uint256, VecM, WriteXdr,
    };

    let horizon = HorizonClient::new(&network.horizon_url);
    let rpc = SorobanRpcClient::new(&network.rpc_url);

    // 1. Get donor account sequence number from Horizon
    let account = horizon
        .get_account(donor)
        .await
        .map_err(|e| StellarAidError::horizon(format!("failed to fetch account: {}", e)))?;
    let seq: u64 = account
        .sequence
        .parse()
        .map_err(|_| StellarAidError::horizon("invalid sequence number"))?;

    // 2. Build ScAddress for donor and contract
    let donor_pk = stellar_strkey::Strkey::from_string(donor)
        .map_err(|_| StellarAidError::validation("invalid donor address"))?;

    let source_account = match &donor_pk {
        stellar_strkey::Strkey::PublicKeyEd25519(pk) => {
            MuxedAccount::Ed25519(Uint256(pk.0))
        }
        _ => return Err(StellarAidError::validation("donor must be a G... address")),
    };

    let donor_addr = match &donor_pk {
        stellar_strkey::Strkey::PublicKeyEd25519(pk) => {
            ScAddress::Account(AccountId(PublicKey::PublicKeyTypeEd25519(Uint256(pk.0))))
        }
        _ => return Err(StellarAidError::validation("donor must be a G... address")),
    };

    let contract_raw = stellar_strkey::Strkey::from_string(donation_contract_id)
        .map_err(|_| StellarAidError::validation("invalid contract id"))?;

    let contract_addr = match &contract_raw {
        stellar_strkey::Strkey::ContractId(h) => ScAddress::Contract(Hash(Uint256(h.0))),
        _ => return Err(StellarAidError::validation("expected a C... contract id")),
    };

    // 3. Build invoke host function args
    let sym_donate = "donate"
        .try_into()
        .map_err(|_| StellarAidError::validation("invalid function name"))?;

    let mut params = ScVec(VecM::default());
    params.0.push(donor_addr);
    params.0.push(ScVal::U64(campaign_id));
    params.0.push(ScVal::I128(soroban_sdk::xdr::Int128Parts {
        lo: amount as u64,
        hi: (amount >> 64) as u64,
    }));

    let host_fn = HostFunction::InvokeHostFunction(InvokeHostFunctionOp {
        contract_id: contract_addr,
        function_name: sym_donate,
        parameters: params,
        auth: VecM::default(),
    });

    // 4. Build transaction
    let op = Operation {
        source_account: None,
        body: OperationBody::InvokeHostFunction(host_fn),
    };

    let mut ops = VecM::default();
    ops.push(op);

    let tx = Transaction {
        source_account,
        fee: 100_000,
        seq_num: SequenceNumber(seq as i64 + 1),
        cond: Preconditions::Time(TimeBounds {
            min_time: 0,
            max_time: 0,
        }),
        memo: Memo::None,
        operations: ops,
        ext: TransactionExt::V0,
    };

    // 5. Encode to XDR for simulation
    let envelope = TransactionEnvelope::EnvelopeTypeTx(tx);
    let xdr = envelope
        .to_xdr_base64()
        .map_err(|e| StellarAidError::validation(format!("XDR encoding failed: {}", e)))?;

    // 6. Simulate to estimate fees
    let simulation = rpc
        .simulate_transaction(&xdr)
        .await
        .map_err(|e| StellarAidError::soroban(format!("simulation failed: {}", e)))?;

    let sim_fee = simulation
        .cost
        .as_ref()
        .and_then(|c| c.get("minResourceFee").and_then(|v| v.as_u64()))
        .unwrap_or(100_000);

    // 7. Rebuild with estimated fee
    let tx_final = Transaction {
        fee: sim_fee + 100,
        ..tx
    };

    let envelope_final = TransactionEnvelope::EnvelopeTypeTx(tx_final);
    envelope_final
        .to_xdr_base64()
        .map_err(|e| StellarAidError::validation(format!("XDR encoding failed: {}", e)))
}
