use std::collections::HashMap;
use std::sync::Mutex;
use thiserror::Error;
use crate::models::donation_status::DonationStatus;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("Storage error: {0}")]
    Storage(String),
}

#[derive(Debug, Clone)]
pub struct NewDonation {
    pub tx_hash: String,
    pub campaign_id: String,
    pub donor_address: String,
    pub amount: u64,
    pub status: DonationStatus,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct Donation {
    pub id: u64,
    pub tx_hash: String,
    pub campaign_id: String,
    pub donor_address: String,
    pub amount: u64,
    pub status: DonationStatus,
    pub created_at: String,
}

/// In-memory donations repository. Replace `inner` with a real DB connection in production.
pub struct DonationsRepo {
    inner: Mutex<(u64, HashMap<String, Donation>)>,
}

impl DonationsRepo {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new((0, HashMap::new())),
        }
    }

    /// Save a donation. Idempotent: returns existing record if `tx_hash` already exists.
    pub fn save_donation(&self, donation: &NewDonation) -> Result<Donation, DbError> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|e| DbError::Storage(e.to_string()))?;
        let (next_id, ref mut map) = *guard;

        if let Some(existing) = map.get(&donation.tx_hash) {
            return Ok(existing.clone());
        }

        let id = next_id + 1;
        guard.0 = id;
        let record = Donation {
            id,
            tx_hash: donation.tx_hash.clone(),
            campaign_id: donation.campaign_id.clone(),
            donor_address: donation.donor_address.clone(),
            amount: donation.amount,
            status: donation.status.clone(),
            created_at: donation.created_at.clone(),
        };
        guard.1.insert(record.tx_hash.clone(), record.clone());
        Ok(record)
    }

    pub fn find_by_tx_hash(&self, tx_hash: &str) -> Result<Option<Donation>, DbError> {
        let guard = self
            .inner
            .lock()
            .map_err(|e| DbError::Storage(e.to_string()))?;
        Ok(guard.1.get(tx_hash).cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_donation(hash: &str) -> NewDonation {
        NewDonation {
            tx_hash: hash.to_string(),
            campaign_id: "camp-1".to_string(),
            donor_address: "GABC".to_string(),
            amount: 100,
            status: DonationStatus::Pending,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn save_and_retrieve() {
        let repo = DonationsRepo::new();
        let d = repo.save_donation(&new_donation("txhash1")).unwrap();
        assert_eq!(d.tx_hash, "txhash1");
        assert_eq!(d.id, 1);
    }

    #[test]
    fn idempotent_on_duplicate_tx_hash() {
        let repo = DonationsRepo::new();
        let d1 = repo.save_donation(&new_donation("txhash2")).unwrap();
        let d2 = repo.save_donation(&new_donation("txhash2")).unwrap();
        assert_eq!(d1.id, d2.id);
    }
}
