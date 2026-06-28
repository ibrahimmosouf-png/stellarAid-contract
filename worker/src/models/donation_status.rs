use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DonationStatus {
    Pending,
    Submitted,
    Confirming,
    Confirmed,
    Failed,
    Refunded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DonationEvent {
    Submit,
    BeginConfirming,
    Confirm,
    Fail,
    Refund,
}

#[derive(Debug, Error, PartialEq, Eq)]
#[error("Invalid transition from {from:?} via {event:?}")]
pub struct TransitionError {
    pub from: DonationStatus,
    pub event: DonationEvent,
}

impl DonationStatus {
    pub fn transition(self, event: DonationEvent) -> Result<DonationStatus, TransitionError> {
        match (&self, &event) {
            (DonationStatus::Pending, DonationEvent::Submit) => Ok(DonationStatus::Submitted),
            (DonationStatus::Submitted, DonationEvent::BeginConfirming) => Ok(DonationStatus::Confirming),
            (DonationStatus::Confirming, DonationEvent::Confirm) => Ok(DonationStatus::Confirmed),
            (DonationStatus::Confirming, DonationEvent::Fail) => Ok(DonationStatus::Failed),
            (DonationStatus::Submitted, DonationEvent::Fail) => Ok(DonationStatus::Failed),
            (DonationStatus::Failed, DonationEvent::Refund) => Ok(DonationStatus::Refunded),
            _ => Err(TransitionError { from: self, event }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_transitions() {
        assert_eq!(
            DonationStatus::Pending.transition(DonationEvent::Submit),
            Ok(DonationStatus::Submitted)
        );
        assert_eq!(
            DonationStatus::Submitted.transition(DonationEvent::BeginConfirming),
            Ok(DonationStatus::Confirming)
        );
        assert_eq!(
            DonationStatus::Confirming.transition(DonationEvent::Confirm),
            Ok(DonationStatus::Confirmed)
        );
        assert_eq!(
            DonationStatus::Confirming.transition(DonationEvent::Fail),
            Ok(DonationStatus::Failed)
        );
        assert_eq!(
            DonationStatus::Failed.transition(DonationEvent::Refund),
            Ok(DonationStatus::Refunded)
        );
    }

    #[test]
    fn invalid_transition_confirmed_to_pending() {
        let result = DonationStatus::Confirmed.transition(DonationEvent::Submit);
        assert!(result.is_err());
    }

    #[test]
    fn invalid_transition_pending_to_confirm() {
        let result = DonationStatus::Pending.transition(DonationEvent::Confirm);
        assert!(result.is_err());
    }
}
