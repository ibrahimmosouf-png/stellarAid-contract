use sdk::horizon::client::{HorizonClient, HorizonError};
use crate::models::donation_status::{DonationEvent, DonationStatus};

#[derive(Debug)]
pub struct VerificationResult {
    pub status: DonationStatus,
    pub ledger: Option<u64>,
    pub created_at: Option<String>,
}

pub async fn cross_check_transaction(
    horizon: &HorizonClient,
    tx_hash: &str,
    current_status: DonationStatus,
) -> Result<VerificationResult, HorizonError> {
    let tx = horizon.get_transaction(tx_hash).await?;

    let event = if tx.successful {
        DonationEvent::Confirm
    } else {
        DonationEvent::Fail
    };

    // Drive through Confirming if still Submitted
    let status = match current_status {
        DonationStatus::Submitted => {
            DonationStatus::Confirming
                .transition(event)
                .unwrap_or(DonationStatus::Failed)
        }
        DonationStatus::Confirming => {
            current_status.transition(event).unwrap_or(DonationStatus::Failed)
        }
        other => other,
    };

    Ok(VerificationResult {
        status,
        ledger: tx.ledger,
        created_at: Some(tx.created_at),
    })
}
